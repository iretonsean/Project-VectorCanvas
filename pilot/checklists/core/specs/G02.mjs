import { R, ok, bad, cannot, norm, btn, rowOfText } from './_common.mjs';
import { TEXT_INPUT_ROLES } from '../lib.mjs';

const FILTER_RE = { All: /^all\b/i, Active: /^active\b/i, Done: /^(done|completed)\b/i };
const COUNT_RE = /(\d+)\s*(?:items?|tasks?|todos?)?\s*(?:left|remaining|active|to do)\b/i;

/** the scripted operations (20) — deterministic, shared by every stack */
export const SCRIPT = [
  ['add', 'Buy milk'], ['add', 'Call mom'], ['add', 'Pay rent'], ['toggle', 'Call mom'], ['add', 'Walk dog'],
  ['filter', 'Active'], ['toggle', 'Buy milk'], ['filter', 'Done'], ['add', 'Read book'], ['filter', 'All'],
  ['delete', 'Call mom'], ['toggle', 'Pay rent'], ['toggle', 'Pay rent'], ['filter', 'Active'], ['delete', 'Walk dog'],
  ['add', 'Water plants'], ['filter', 'All'], ['toggle', 'Read book'], ['filter', 'Done'], ['delete', 'Buy milk'],
];

export default {
  id: 'G02', category: '4 Logic-heavy', baseline: 'B2',
  intent: 'To-do app: add, complete, delete, filter All/Active/Done, count of active items',
  fixture: ['The app has a text field labelled "New task" and an "Add" button. Each task shows a checkbox, its text and a "Delete" button. Three filter buttons are named "All", "Active" and "Done". A line shows the number of tasks that are not done, written "N items left" (for example "3 items left", "1 item left").'],
  conventions: [
    'new-task field = first visible text input (preferring a name matching /task|todo|new|add/i); add = button named /^add/i; filters = button/link/tab/radio named /^all/i, /^active/i, /^(done|completed)/i; per-task controls are found in the task\'s row (the largest ancestor of the task text that holds a checkbox/button and no other task text): checkbox = completes/uncompletes, a button named /delete|remove|×|✕|x/i = deletes',
    'count = first visible non-interactive text matching /(\\d+) (items|tasks)? (left|remaining|active)/i; it always counts ALL tasks that are not done, regardless of the filter',
    'visible tasks = the set of known task texts that are visible; order is ignored',
  ],
  items: [
    {
      id: 'G02.1', kind: 'A', trace: true, g2: 'Counts correct after any sequence of 20 scripted operations (state trace)',
      op: ['Apply the 20 scripted operations (SCRIPT in `core/specs/G02.mjs`: add, toggle (complete/uncomplete via the task checkbox), delete, filter) on a fresh load at 1280 px. After EACH operation compare against the reference model: (i) the set of visible task texts equals the model\'s tasks for the current filter (All = all, Active = not done, Done = done); (ii) the "items left" count equals the number of not-done tasks.',
        'Any operation whose control cannot be found, or any mismatch, fails the item; the detail names the step.'],
      feedback: 'Not met: after any sequence of add / complete / delete / filter operations the visible tasks and the count of active items must be correct.',
      async run(ctx) {
        let s = await ctx.at(1280);
        const model = []; // {text, done}
        let filter = 'All';
        const names = SCRIPT.filter(o => o[0] === 'add').map(o => o[1]);
        const check = async (step) => {
          const sn = await ctx.now();
          const vis = names.filter(n => sn.byText(n).length > 0);
          const exp = model.filter(t => filter === 'All' || (filter === 'Active' ? !t.done : t.done)).map(t => t.text);
          if (vis.slice().sort().join('|') !== exp.slice().sort().join('|')) return `step ${step}: visible [${vis}] expected [${exp}]`;
          const cn = sn.nodes.find(n => sn.isVisible(n) && n.text && !(n.role && ['button', 'link', 'tab'].includes(n.role)) && COUNT_RE.test(n.text));
          if (!cn) return `step ${step}: no "items left" count found`;
          const want = model.filter(t => !t.done).length; const got = +COUNT_RE.exec(cn.text)[1];
          if (got !== want) return `step ${step}: count ${got} expected ${want}`;
          return null;
        };
        for (const [i, [op, arg]] of SCRIPT.entries()) {
          const step = `${i + 1} (${op} ${arg})`;
          s = await ctx.now();
          if (op === 'add') {
            const ins = s.byRole([...TEXT_INPUT_ROLES]);
            const inp = ins.find(n => /task|todo|new|add/i.test(n.name ?? '')) ?? ins[0];
            const ab = btn(s, /^add/i);
            if (!inp || !ab) return cannot(`step ${step}: input or Add button not found`);
            await ctx.type(inp, arg, { replace: true }); await ctx.activate(ab);
            model.push({ text: arg, done: false });
          } else if (op === 'filter') {
            const f = s.byRole(['button', 'link', 'tab', 'radio'], FILTER_RE[arg])[0];
            if (!f) return cannot(`step ${step}: filter control not found`);
            await ctx.activate(f); filter = arg;
          } else {
            const others = names.filter(n => n !== arg);
            const r = rowOfText(s, arg, others);
            if (!r) return cannot(`step ${step}: task row not found`);
            if (op === 'toggle') {
              const cb = r.within('checkbox')[0]; if (!cb) return cannot(`step ${step}: no checkbox in row`);
              await ctx.activate(cb); const t = model.find(t => t.text === arg); t.done = !t.done;
            } else {
              const d = r.within('button').find(b => /delete|remove|×|✕|✖|\bx\b|trash/i.test(b.name ?? ''));
              if (!d) return cannot(`step ${step}: no delete button in row`);
              await ctx.activate(d); model.splice(model.findIndex(t => t.text === arg), 1);
            }
          }
          const err = await check(step);
          if (err) return bad(err);
        }
        return ok();
      },
    },
  ],
};
