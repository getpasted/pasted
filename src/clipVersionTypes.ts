import type { EffectiveVisualLabels } from './components/visualLabelModel';

export interface ClipVersion {
  id: number;
  clip_id: number;
  text_content: string;
  action_kind?: string | null;
  action_label?: string | null;
  restores_organization?: boolean;
  visual_labels?: EffectiveVisualLabels | null;
  is_current?: boolean;
  is_original?: boolean;
  created_at: string;
}
