// Measure complete text before drawing; never truncate diary details.
export function layoutDiaryDetails(ctx, text, maxWidth = 820) {
  const lines = [];
  let line = '';
  for (const character of text) {
    if (line && ctx.measureText(line + character).width > maxWidth) {
      lines.push(line);
      line = character;
    } else {
      line += character;
    }
  }
  if (line) lines.push(line);
  const lineHeight = 44;
  const detailY = 550;
  const closingY = detailY + Math.max(0, lines.length - 1) * lineHeight + 110;
  const catY = closingY + 70;
  return { lines, lineHeight, detailY, closingY, catY, height: Math.max(1350, catY + 325 + 140) };
}
