interface BrowserBin {
  id: number;
  smart_rule?: string | null;
}

interface BrowserClip {
  source: string;
  bin_ids: number[];
}

interface SmartCondition {
  type?: string;
  target?: string;
  operator?: string;
  value?: string;
}

export function browserClipMatchesBin(clip: BrowserClip, bin: BrowserBin | undefined) {
  if (!bin) return false;
  if (clip.bin_ids.includes(bin.id)) return true;
  if (!bin.smart_rule) return false;
  try {
    const rule = JSON.parse(bin.smart_rule) as {
      conditions?: SmartCondition[];
      match?: string;
      match_mode?: string;
    };
    const matches = (rule.conditions ?? []).map((condition) => {
      if ((condition.target ?? condition.type) !== 'source') return false;
      const source = clip.source.toLocaleLowerCase();
      const value = String(condition.value ?? '').toLocaleLowerCase();
      return condition.operator === 'is' ? source === value : source.includes(value);
    });
    return (rule.match_mode ?? rule.match) === 'all'
      ? matches.length > 0 && matches.every(Boolean)
      : matches.some(Boolean);
  } catch {
    return false;
  }
}
