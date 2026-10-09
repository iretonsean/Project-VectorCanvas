import { R, ok, bad, cannot, norm, btn, rowOfText, labelAmount, parseMoney } from './_common.mjs';
import { TEXT_INPUT_ROLES } from '../lib.mjs';

export const FIXTURE = { items: [['Notebook', 499], ['Pen', 125], ['Backpack', 1990]], code: 'SAVE10', discountPct: 10, taxBp: 825 };
/** exact integer-cent reference: discount and tax rounded half-up to the cent; total = subtotal − discount + tax */
export function expected(qty, promo) {
  const sub = FIXTURE.items.reduce((a, [n, c]) => a + c * (qty[n] ?? 0), 0);
  const disc = promo ? Math.floor((sub * FIXTURE.discountPct + 50) / 100) : 0;
  const base = sub - disc;
  const tax = Math.floor((base * FIXTURE.taxBp + 5000) / 10000);
  return { sub, disc, tax, total: base + tax };
}
const fmt = c => '$' + (c / 100).toFixed(2);
const INC = /increase|plus|^\+$|more/i, DEC = /decrease|minus|^[-−–]$|fewer|less/i, REM = /remove|delete/i;

async function readTotals(ctx) {
  const s = await ctx.now();
  return { s, sub: labelAmount(s, /^subtotal/i), tax: labelAmount(s, /^tax/i), total: labelAmount(s, /^total\b/i) };
}
function cmp(step, got, exp) {
  for (const k of ['sub', 'tax', 'total']) if (got[k] !== exp[k]) return `${step}: ${k} shows ${got[k] == null ? 'nothing' : fmt(got[k])}, expected ${fmt(exp[k])}`;
  return null;
}
async function press(ctx, itemName, re, others) {
  const s = await ctx.now(); const r = rowOfText(s, itemName, others);
  if (!r) return `row for "${itemName}" not found`;
  const b = r.within('button').find(x => re.test(x.name ?? '')); if (!b) return `button ${re} not found for "${itemName}"`;
  await ctx.activate(b); return null;
}
async function applyCode(ctx, code) {
  const s = await ctx.now();
  const inp = s.byRole([...TEXT_INPUT_ROLES], /promo|code|coupon/i)[0]; const ap = btn(s, /apply|redeem/i);
  if (!inp || !ap) return 'promo field or Apply button not found';
  await ctx.type(inp, code, { replace: true }); await ctx.activate(ap); return null;
}

export default {
  id: 'G04', category: '4 Logic-heavy', baseline: 'B2',
  intent: 'Shopping cart: quantities, remove, promo code (one valid code), subtotal/tax/total',
  fixture: ['The cart has three lines: "Notebook" $4.99, "Pen" $1.25 and "Backpack" $19.90, each starting at quantity 1, with a button named "Increase quantity" and one named "Decrease quantity" (the quantity never goes below 1) and a button named "Remove" inside its row. Below the lines there is a text field labelled "Promo code" with an "Apply" button. The only valid code is SAVE10, which takes 10% off the subtotal; any other code shows an error message containing the word "Invalid". The summary shows four lines labelled "Subtotal" (sum of all lines before discount), "Discount", "Tax" (8.25% of subtotal minus discount, rounded half-up to the cent) and "Total" (subtotal minus discount plus tax). All money is shown in dollars with two decimals, like "$4.99", and must be exact decimal arithmetic with no floating-point rounding errors.'],
  conventions: [
    'rows: line text = exact item name; per-row buttons named /increase|plus|+/ , /decrease|minus|−/ , /remove|delete/',
    'amounts: for labels Subtotal, Tax, Total the amount is in the same text node or the next money text in reading order within the same container',
    'promo field = text input named /promo|code|coupon/i; apply = button named /apply|redeem/i',
    'reference model in `core/specs/G04.mjs` uses integer cents with half-up rounding',
  ],
  items: [
    {
      id: 'G04.1', kind: 'A', trace: true, g2: 'Arithmetic exact in decimal',
      op: ['Fresh load. Check Subtotal/Tax/Total against the exact reference at four states: (1) initial (Notebook 1, Pen 1, Backpack 1); (2) after Increase on Backpack (qty 2); (3) after applying promo code SAVE10; (4) after Remove on Pen.',
        'A mismatch of even one cent in any of the three lines fails the item.'],
      feedback: 'Not met: the subtotal, tax and total must be exact in decimal arithmetic (to the cent) after every change.',
      async run(ctx) {
        await ctx.at(1280);
        const names = FIXTURE.items.map(i => i[0]);
        let t = await readTotals(ctx); const q = { Notebook: 1, Pen: 1, Backpack: 1 };
        if (t.sub == null || t.tax == null || t.total == null) return cannot('Subtotal/Tax/Total amounts not found');
        let e = cmp('initial', t, expected(q, false)); if (e) return bad(e);
        e = await press(ctx, 'Backpack', INC, names.filter(n => n !== 'Backpack')); if (e) return cannot(e);
        q.Backpack = 2; t = await readTotals(ctx); e = cmp('after Increase Backpack', t, expected(q, false)); if (e) return bad(e);
        e = await applyCode(ctx, FIXTURE.code); if (e) return cannot(e);
        t = await readTotals(ctx); e = cmp('after SAVE10', t, expected(q, true)); if (e) return bad(e);
        e = await press(ctx, 'Pen', REM, names.filter(n => n !== 'Pen')); if (e) return cannot(e);
        q.Pen = 0; t = await readTotals(ctx); e = cmp('after Remove Pen', t, expected(q, true)); if (e) return bad(e);
        return ok();
      },
    },
    {
      id: 'G04.2', kind: 'A', trace: true, g2: 'Invalid code error',
      op: ['Fresh load; Increase Backpack (qty 2) so the totals are non-trivial; enter promo code "WRONG" and press Apply. A visible message matching /invalid|not valid|unknown|not recognised|not recognized|incorrect/i that was not visible before must appear (an `alert`/`status` node or plain text), AND Subtotal/Tax/Total must be unchanged (no discount applied).'],
      feedback: 'Not met: an invalid promo code must show an error message and must not change the totals.',
      async run(ctx) {
        await ctx.at(1280);
        const names = FIXTURE.items.map(i => i[0]);
        let e = await press(ctx, 'Backpack', INC, names.filter(n => n !== 'Backpack')); if (e) return cannot(e);
        const before = await readTotals(ctx); const seen = new Set(before.s.textNodes().map(n => n.text));
        e = await applyCode(ctx, 'WRONG'); if (e) return cannot(e);
        const after = await readTotals(ctx);
        const msg = after.s.textNodes().find(n => /invalid|not valid|unknown|not recognised|not recognized|incorrect/i.test(n.text) && !seen.has(n.text));
        if (!msg) return bad('no error message after an invalid code');
        if (before.sub !== after.sub || before.tax !== after.tax || before.total !== after.total) return bad('totals changed after an invalid code');
        return ok();
      },
    },
  ],
};
