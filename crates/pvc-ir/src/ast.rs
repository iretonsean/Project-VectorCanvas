//! The canonical PVC-IR module AST (G4 §3). One type tree serves both the terse expansion
//! output and the loaded signed artifact. Everything in it is explicit: no defaults are implied.
//!
//! No floats anywhere: lengths are fixed-point (1/64 logical px, the D3 snapping unit).

use crate::diag::Span;

pub type Id = String;

macro_rules! str_enum {
    ($(#[$m:meta])* $name:ident { $($var:ident = $s:literal),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name { $($var),* }
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$var),*];
            pub fn as_str(self) -> &'static str { match self { $($name::$var => $s),* } }
            pub fn from_name(s: &str) -> Option<$name> { match s { $($s => Some($name::$var),)* _ => None } }
            pub fn names() -> &'static [&'static str] { &[$($s),*] }
        }
    };
}


/// Length in 1/64 logical px.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Len(pub i32);

impl Len {
    pub const ZERO: Len = Len(0);
    pub fn from_px(px: i32) -> Len {
        Len(px * 64)
    }
    pub fn raw(self) -> i32 {
        self.0
    }
    /// Logical px as f32 (hosts convert; the AST itself stays integer).
    pub fn px(self) -> f32 {
        self.0 as f32 / 64.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Edges {
    pub top: Len,
    pub right: Len,
    pub bottom: Len,
    pub left: Len,
}
impl Edges {
    pub fn all(l: Len) -> Edges {
        Edges { top: l, right: l, bottom: l, left: l }
    }
    pub fn xy(x: Len, y: Len) -> Edges {
        Edges { top: y, right: x, bottom: y, left: x }
    }
}

/// `#rrggbb[aa]` or a theme token (resolve with [`Module::resolve_color`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Color {
    Token(String),
    Rgba(u8, u8, u8, u8),
}

// ---------------------------------------------------------------- module

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub header: Header,
    pub manifest: Manifest,
    pub theme: Theme,
    pub strings: Strings,
    pub state: Vec<StateDecl>,
    pub derived: Vec<Derived>,
    pub effects: Vec<Effect>,
    pub root: Node,
    /// Node id -> terse source span. **Not canonical**: never serialised, stripped on publish (G4 §3).
    pub provenance: Vec<(Id, Span)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub ir: (u16, u16),
    pub std: (u16, u16),
    pub dialect: Option<String>,
    pub min_runtime: (u16, u16),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// Declared locales; the first is the default/authoring locale.
    pub locales: Vec<String>,
    /// Fallback order = declaration order.
    pub fonts: Vec<FontEntry>,
    pub caps: Vec<Capability>,
    /// Supported viewport width/height range, logical px (min_w, max_w, min_h, max_h).
    pub viewport: (u32, u32, u32, u32),
    /// Supported text-scale range in percent (min, max).
    pub text_scale: (u16, u16),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontEntry {
    pub family: String,
    pub weight: u16,
    /// Optional script tag (e.g. `Hans`): this face is a fallback for that script.
    pub script: Option<String>,
    /// Content hash (lowercase hex sha256) of the font file bytes: authoritative.
    pub sha256: String,
    /// Informational file name hint (not authoritative).
    pub file: String,
    /// Face index inside a collection (.ttc); 0 for plain files.
    pub index: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    /// `<kind>.<name>`, e.g. `net.main`.
    pub id: String,
    pub kind: String,
    pub hosts: Vec<String>,
    pub methods: Vec<String>,
    pub max_request_bytes: u64,
    pub max_daily_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    /// Sorted by token name.
    pub colors: Vec<(String, [u8; 4])>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Strings {
    /// In manifest locale order.
    pub tables: Vec<LocaleTable>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocaleTable {
    pub locale: String,
    /// Sorted by key (bytewise). Texts are NFC.
    pub entries: Vec<(String, String)>,
}

// ---------------------------------------------------------------- behaviour

str_enum!(
    /// State types (G4 §7.1 subset: bool, int, decimal, text, enum, list of records).
    TyKind { Bool = "bool", Int = "int", Decimal = "decimal", Text = "text", Enum = "enum", List = "list" }
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    Bool,
    Int,
    Text,
    Enum(Vec<String>),
    /// Fixed-point decimal with exactly 2 fractional digits (stored as hundredths in an i64).
    Decimal,
    /// Bounded list of records; `max` is mandatory (`PVC-E-UNBOUNDED`).
    List { fields: Vec<(String, Ty)>, max: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Text(String),
    Enum(String),
    /// Hundredths: `Decimal(499)` is 4.99.
    Decimal(i64),
    /// Rows in order; each row lists `(field, value)` in the declared field order.
    List(Vec<Vec<(String, Value)>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateDecl {
    pub name: String,
    pub ty: Ty,
    pub init: Value,
}

/// A named pure expression over state and earlier derived values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derived {
    pub name: String,
    pub expr: Expr,
}

str_enum!(CmpOp { Eq = "==", Ne = "!=", Lt = "<", Le = "<=", Gt = ">", Ge = ">=" });

str_enum!(
    /// Pure functions allowed in guards (G4 §7.3: `is-empty`, `length`, `contains`; plus `is-email`).
    Func {
        IsEmpty = "is-empty", Length = "length", Contains = "contains", IsEmail = "is-email",
        Count = "count", Sum = "sum", Percent = "percent"
    }
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Bool(bool),
    Int(i64),
    /// Text literal (display data / comparison operand, not a strings-table key).
    Str(String),
    /// State field or derived value.
    Var(String),
    /// Enum variant literal.
    Variant(String),
    /// Decimal literal in hundredths (`4.99` -> `Dec(499)`).
    Dec(i64),
    /// `+ - *` on ints and decimals (int * decimal -> decimal; no decimal * decimal).
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Cmp(CmpOp, Box<Expr>, Box<Expr>),
    Call(Func, Vec<Expr>),
}

str_enum!(Event { Press = "press", Change = "change", Submit = "submit" });

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handler {
    pub event: Event,
    /// If present and false, the handler does nothing.
    pub guard: Option<Expr>,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// state field = expr (type-checked). The target may be `alias.field` inside a `view` template
    /// (updates that row in place).
    Assign(String, Expr),
    /// `append <list> { field = expr … }`: add a row (ignored when the list is at its `max`).
    Append(String, Vec<(String, Expr)>),
    /// `remove <alias>`: remove the row bound to the enclosing `view`'s alias.
    Remove(String),
    /// Open a URL through the `open-url` capability (dispatcher stub: logs only in this session).
    OpenUrl(String),
    /// Dispatch the named effect through the capability dispatcher (never a network call in this session).
    Emit(String),
    /// Raise `submit` on the node with this id (runs its `submit` handler).
    Submit(Id),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    pub name: String,
    /// Capability id declared in the manifest.
    pub cap: String,
}

// ---------------------------------------------------------------- nodes

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: Id,
    pub kind: NodeKind,
    pub layout: Layout,
    pub paint: Paint,
    pub sem: Semantics,
    /// Node (and its subtree) exists, is laid out, painted and in the a11y tree only while this is true.
    pub show: Option<Expr>,
    pub on: Vec<Handler>,
    /// Breakpoint overrides ordered `Md` then `Sm` (not serialised when empty).
    pub at: Vec<AtRule>,
    pub children: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    Surface,
    Box,
    Text(TextProps),
    TextInput(InputProps),
    /// Painted leaf (rect / rounded / circle by `paint.shape`): icons, illustrations, dividers.
    Shape,
    /// Out-of-flow overlay (dialog): centred over the surface while `show` is true; traps focus.
    Layer,
    /// Exactly one child is present: `cases[i]` names `children[i]`; the child whose case equals the value of `on`.
    Slot(SlotProps),
    /// One copy of the children (the template) per row of `list` that passes `filter`; the row is bound to `alias`.
    View(ViewProps),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotProps {
    pub on: Expr,
    pub cases: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewProps {
    pub list: String,
    pub alias: String,
    pub filter: Option<Expr>,
}

impl NodeKind {
    pub fn name(&self) -> &'static str {
        match self {
            NodeKind::Surface => "surface",
            NodeKind::Box => "box",
            NodeKind::Text(_) => "text",
            NodeKind::TextInput(_) => "text-input",
            NodeKind::Shape => "shape",
            NodeKind::Layer => "layer",
            NodeKind::Slot(_) => "slot",
            NodeKind::View(_) => "view",
        }
    }
}

str_enum!(Dir { Row = "row", Col = "col", Grid = "grid" });
str_enum!(Align { Start = "start", Center = "center", End = "end", Stretch = "stretch" });
str_enum!(Justify { Start = "start", Center = "center", End = "end", Between = "between" });

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Size {
    /// Content size.
    Hug,
    /// Main axis: share the remaining space equally among `Fill` siblings. Cross axis: stretch to the parent content box.
    Fill,
    /// Fixed. In a `col`/`surface` parent a fixed width is capped at the parent's content width.
    Px(Len),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub dir: Dir,
    pub gap: Len,
    pub pad: Edges,
    /// Cross-axis alignment of children.
    pub align: Align,
    /// Main-axis distribution of children.
    pub justify: Justify,
    pub w: Size,
    pub h: Size,
    /// Number of equal `1fr` columns when `dir` is `Grid`; 0 otherwise.
    pub cols: u16,
}

/// Viewport-width breakpoints (logical px): `Md` = width <= 768, `Sm` = width <= 480. `Sm` overrides `Md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Bp {
    Md,
    Sm,
}
impl Bp {
    pub fn as_str(self) -> &'static str {
        match self {
            Bp::Md => "md",
            Bp::Sm => "sm",
        }
    }
    pub fn from_name(s: &str) -> Option<Bp> {
        match s {
            "md" => Some(Bp::Md),
            "sm" => Some(Bp::Sm),
            _ => None,
        }
    }
    /// Largest viewport width (logical px) at which this breakpoint applies.
    pub fn max_width(self) -> u32 {
        match self {
            Bp::Md => 768,
            Bp::Sm => 480,
        }
    }
}

/// Layout fields a breakpoint may override. `None` = keep the base value.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LayoutPatch {
    pub dir: Option<Dir>,
    pub cols: Option<u16>,
    pub gap: Option<Len>,
    pub pad: Option<Edges>,
    pub align: Option<Align>,
    pub justify: Option<Justify>,
    pub w: Option<Size>,
    pub h: Option<Size>,
}

/// Breakpoint-keyed override set (G4 §5.3 "constraint sets keyed by viewport-width ranges").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtRule {
    pub bp: Bp,
    pub patch: LayoutPatch,
    /// `Some(false)` hides the node at this breakpoint (in addition to `show`).
    pub show: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Corner radius; `Len(0)` is a plain rectangle.
    RoundedRect(Len),
    /// Circle inscribed in the node's (square) box.
    Circle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Border {
    pub width: Len,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paint {
    pub fill: Option<Color>,
    pub border: Option<Border>,
    pub shape: Shape,
}

str_enum!(TextAlign { Start = "start", Center = "center", End = "end" });

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextProps {
    /// Strings-table key.
    pub key: String,
    /// Font family id from the manifest.
    pub family: String,
    pub size: Len,
    pub weight: u16,
    pub line_height: Len,
    pub color: Color,
    pub align: TextAlign,
    /// Named holes `{name}` of the string bound to expressions (ints print as digits, decimals with 2 fraction digits, text verbatim).
    pub args: Vec<(String, Expr)>,
    /// `Some(expr)`: display the value of the expression verbatim instead of a strings entry (`key` is then empty).
    pub value: Option<Expr>,
}

str_enum!(InputVariant {
    Single = "single", Email = "email", Password = "password", Search = "search", Multiline = "multiline"
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputProps {
    pub variant: InputVariant,
    /// `text` state field edited by this input.
    pub bind: String,
    /// While true the content is masked (password). `Bool(false)` for non-password.
    pub masked: Expr,
    pub family: String,
    pub size: Len,
    pub line_height: Len,
    pub color: Color,
}

// ---------------------------------------------------------------- semantics

str_enum!(
    /// PVC roles used by the A1 subset (G4 §8.1). `aria()` gives the web-mirror role.
    Role {
        Button = "button", Link = "link", Checkbox = "checkbox", Switch = "switch",
        TextField = "text-field", PasswordField = "password-field", EmailField = "email-field",
        Group = "group", Container = "container", Image = "image", Label = "label",
        Paragraph = "paragraph", Heading = "heading", Status = "status", Alert = "alert",
        Form = "form", Main = "main", Navigation = "navigation", Region = "region",
        Radio = "radio", RadioGroup = "radio-group", TextArea = "text-area", SearchField = "search-field",
        List = "list", ListItem = "list-item", TabList = "tab-list", Tab = "tab", TabPanel = "tab-panel",
        Dialog = "dialog", AlertDialog = "alert-dialog",
    }
);

impl Role {
    pub fn is_interactive(self) -> bool {
        matches!(
            self,
            Role::Button | Role::Link | Role::Checkbox | Role::Switch | Role::TextField
                | Role::PasswordField | Role::EmailField | Role::Radio | Role::TextArea
                | Role::SearchField | Role::Tab
        )
    }
    /// Roles that must carry an accessible name (`PVC-E-A11Y-NAME`). Text-bearing roles
    /// (`label`, `paragraph`, `heading`, `status`, `alert`) take their name from content.
    pub fn needs_name(self) -> bool {
        self.is_interactive()
            || matches!(self, Role::Region | Role::RadioGroup | Role::Dialog | Role::AlertDialog)
    }
    pub fn aria(self) -> &'static str {
        match self {
            Role::Button => "button",
            Role::Link => "link",
            Role::Checkbox => "checkbox",
            Role::Switch => "switch",
            Role::TextField | Role::PasswordField | Role::EmailField => "textbox",
            Role::Group => "group",
            Role::Container => "presentation",
            Role::Image => "img",
            Role::Label => "generic",
            Role::Paragraph => "paragraph",
            Role::Heading => "heading",
            Role::Status => "status",
            Role::Alert => "alert",
            Role::Form => "form",
            Role::Main => "main",
            Role::Navigation => "navigation",
            Role::Region => "region",
            Role::Radio => "radio",
            Role::RadioGroup => "radiogroup",
            Role::TextArea => "textbox",
            Role::SearchField => "searchbox",
            Role::List => "list",
            Role::ListItem => "listitem",
            Role::TabList => "tablist",
            Role::Tab => "tab",
            Role::TabPanel => "tabpanel",
            Role::Dialog => "dialog",
            Role::AlertDialog => "alertdialog",
        }
    }
}

str_enum!(Live { Off = "off", Polite = "polite", Assertive = "assertive" });

/// Where a node's accessible name comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Name {
    /// Strings-table key.
    Key(String),
    /// The text content of another node (by id), e.g. an avatar named by the person's name text.
    FromNode(Id),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Semantics {
    pub role: Role,
    /// `decorative` hides the node from assistive technology.
    pub decorative: bool,
    pub name: Option<Name>,
    pub description: Option<String>,
    pub heading_level: Option<u8>,
    pub live: Live,
    pub required: bool,
    pub disabled: Option<Expr>,
    pub checked: Option<Expr>,
    pub pressed: Option<Expr>,
    pub invalid: Option<Expr>,
    /// `tab` selection state (serialised only when present).
    pub selected: Option<Expr>,
    pub labelled_by: Option<Id>,
    pub described_by: Vec<Id>,
    pub error_message: Option<Id>,
}

impl Semantics {
    pub fn new(role: Role) -> Semantics {
        Semantics {
            role,
            decorative: false,
            name: None,
            description: None,
            heading_level: None,
            live: Live::Off,
            required: false,
            disabled: None,
            checked: None,
            pressed: None,
            invalid: None,
            selected: None,
            labelled_by: None,
            described_by: Vec::new(),
            error_message: None,
        }
    }
}

// ---------------------------------------------------------------- helpers

impl Module {
    /// Depth-first, document-order walk (the order the validator and a11y tree use).
    pub fn walk<'a>(&'a self, f: &mut dyn FnMut(&'a Node, usize)) {
        fn go<'a>(n: &'a Node, d: usize, f: &mut dyn FnMut(&'a Node, usize)) {
            f(n, d);
            for c in &n.children {
                go(c, d + 1, f);
            }
        }
        go(&self.root, 0, f);
    }

    pub fn find_node(&self, id: &str) -> Option<&Node> {
        let mut out = None;
        self.walk(&mut |n, _| {
            if out.is_none() && n.id == id {
                out = Some(n);
            }
        });
        out
    }

    pub fn string(&self, locale: &str, key: &str) -> Option<&str> {
        let t = self.strings.tables.iter().find(|t| t.locale == locale)?;
        t.entries
            .binary_search_by(|(k, _)| k.as_str().cmp(key))
            .ok()
            .map(|i| t.entries[i].1.as_str())
    }

    /// Default (authoring) locale: the first declared.
    pub fn default_locale(&self) -> &str {
        self.manifest.locales.first().map(|s| s.as_str()).unwrap_or("en")
    }

    pub fn resolve_color(&self, c: &Color) -> Option<[u8; 4]> {
        match c {
            Color::Rgba(r, g, b, a) => Some([*r, *g, *b, *a]),
            Color::Token(t) => self
                .theme
                .colors
                .iter()
                .find(|(n, _)| n == t)
                .map(|(_, v)| *v),
        }
    }

    pub fn span_of(&self, id: &str) -> Option<Span> {
        self.provenance.iter().find(|(i, _)| i == id).map(|(_, s)| *s)
    }
}

impl Node {
    pub fn is_interactive(&self) -> bool {
        self.sem.role.is_interactive() && !self.sem.decorative
    }
}
