import test from 'node:test';
import assert from 'node:assert/strict';
import { summarizeWeek, dateLabel } from '../ui/hydration-week.js';
const day = (date, totalMl = 0, recordCount = 0) => ({ localDate: date, totalMl, recordCount, records: [] });
const week = (amounts) => {
  const days = amounts.map((amount, i) => day(`2026-10-${String(i + 1).padStart(2, '0')}`, amount, amount ? 1 : 0));
  return { days, totalMl: amounts.reduce((a, b) => a + b, 0), recordedDays: amounts.filter(Boolean).length };
};
test('empty and sparse histories never claim a fall in water intake', () => {
  assert.match(summarizeWeek(week([0,0,0,0,0,0,0])), /还没有喝水记录/);
  const text = summarizeWeek(week([0,300,0,0,0,0,200]));
  assert.match(text, /2天/); assert.match(text, /500 ml/); assert.doesNotMatch(text, /少|下降|达标/);
});
test('comparison uses two completed days and does not mistake today for a full day', () => {
  const text = summarizeWeek(week([200,300,500,400,800,600,10]));
  assert.match(text, /昨天比前天少记录了200 ml/); assert.match(text, /今天还在累计/);
  assert.doesNotMatch(text, /590/);
  assert.match(summarizeWeek(week([0,0,0,0,300,300,0])), /水量相同/);
});
test('labels preserve stored local dates without UTC conversion', () => {
  assert.equal(dateLabel('2026-01-01', '2026-01-01'), '今天');
  assert.equal(dateLabel('2025-12-31', '2026-01-01'), '12/31');
});
