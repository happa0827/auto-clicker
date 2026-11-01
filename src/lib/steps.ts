const inRange = <T>(items: readonly T[], index: number) =>
  Number.isInteger(index) && index >= 0 && index < items.length;

export function addStep<T>(steps: readonly T[], step: T): T[] {
  return [...steps, step];
}

export function removeStep<T>(steps: readonly T[], index: number): T[] {
  if (!inRange(steps, index)) return [...steps];
  return steps.filter((_, i) => i !== index);
}

export function moveStep<T>(steps: readonly T[], from: number, to: number): T[] {
  if (!inRange(steps, from)) return [...steps];
  const target = Math.min(Math.max(to, 0), steps.length - 1);
  const next = steps.filter((_, i) => i !== from);
  next.splice(target, 0, steps[from]);
  return next;
}

export function updateStep<T>(steps: readonly T[], index: number, step: T): T[] {
  if (!inRange(steps, index)) return [...steps];
  return steps.map((s, i) => (i === index ? step : s));
}
