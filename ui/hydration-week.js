// This summary describes saved records; it does not infer health or unlogged water.
export function summarizeWeek(week) {
  const days = week.days || [];
  const recorded = days.filter(day => day.recordCount > 0);
  if (!recorded.length) return '这7天还没有喝水记录，下一杯水，米奇陪你一起记～';
  const total = week.totalMl.toLocaleString('zh-CN');
  const yesterday = days[5];
  const before = days[4];
  if (yesterday?.recordCount && before?.recordCount) {
    const delta = yesterday.totalMl - before.totalMl;
    const change = delta === 0 ? '昨天与前天记录的水量相同' : `昨天比前天${delta > 0 ? '多' : '少'}记录了${Math.abs(delta).toLocaleString('zh-CN')} ml`;
    return `近7天共记录${total} ml，${change}，今天还在累计～`;
  }
  return `近7天有${recorded.length}天留下喝水记录，共${total} ml，米奇记得每一次咕噜噜～`;
}

export function dateLabel(date, today) {
  return date === today ? '今天' : `${Number(date.slice(5, 7))}/${Number(date.slice(8, 10))}`;
}

export function renderWeekChart(container, days, selectedDate, onSelect) {
  container.replaceChildren();
  const peak = Math.max(500, ...days.map(day => day.totalMl));
  const ceiling = Math.ceil(peak / 500) * 500;
  const today = days.at(-1)?.localDate;
  for (const day of days) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'week-day';
    button.classList.toggle('selected', day.localDate === selectedDate);
    button.classList.toggle('unrecorded', !day.recordCount);
    button.setAttribute('aria-pressed', String(day.localDate === selectedDate));
    button.setAttribute('aria-label', `${day.localDate}，${day.recordCount ? `${day.totalMl}毫升，${day.recordCount}次` : '未记录'}，查看明细`);
    button.title = button.getAttribute('aria-label');
    const plot = document.createElement('span'); plot.className = 'week-day-plot';
    const value = document.createElement('span'); value.className = 'week-day-value';
    value.textContent = day.recordCount ? String(day.totalMl) : '—';
    const bar = document.createElement('span'); bar.className = 'week-day-bar';
    bar.style.height = `${Math.max(3, day.totalMl / ceiling * 86)}px`;
    plot.append(value, bar);
    const label = document.createElement('span'); label.className = 'week-day-label'; label.textContent = dateLabel(day.localDate, today);
    button.append(plot, label);
    button.addEventListener('click', () => onSelect(day.localDate));
    container.append(button);
  }
  return ceiling;
}
