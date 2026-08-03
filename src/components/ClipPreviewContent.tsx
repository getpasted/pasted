import { Check, Copy, Palette, ScanText, Sparkles } from 'lucide-react';
import type { ClipItem } from '../types';
import type { ColorFormats } from '../utils/color';

interface ClipPreviewContentProps {
  clip: ClipItem;
  displayText: string;
  colorData: ColorFormats | null;
  resolvedImageBase64: string | null;
  copiedFormat: string | null;
  isOcrLoading: boolean;
  readOnly?: boolean;
  onColorChange: (value: string) => void;
  onCopyFormat: (label: string, value: string) => void;
  onRunOCR: () => void;
}

export function ClipPreviewContent({
  clip,
  displayText,
  colorData,
  resolvedImageBase64,
  copiedFormat,
  isOcrLoading,
  readOnly = false,
  onColorChange,
  onCopyFormat,
  onRunOCR,
}: ClipPreviewContentProps) {
  return (
    <>
        {/* Color Palette Card Mode */}
        {colorData ? (
          <div className="p-6 bg-[#161820] rounded-2xl border border-gray-800 shadow-2xl space-y-6">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2 text-amber-400 font-sans font-semibold text-xs">
                <Palette className="w-4 h-4" />
                <span>Color Inspector & Swatch Card</span>
              </div>
              <span className="text-[10px] text-gray-500 font-mono">WCAG Contrast Rated</span>
            </div>

            <div className="flex items-center space-x-6">
              <div
                className="w-24 h-24 rounded-2xl border-2 border-white/20 shadow-2xl transition-all duration-300 relative group shrink-0"
                style={{
                  backgroundColor: colorData.hex,
                  boxShadow: `0 12px 32px ${colorData.hex}44`,
                }}
              >
                <input
                  type="color"
                  value={colorData.hex}
                  onChange={(e) => onColorChange(e.target.value)}
                  className="absolute inset-0 opacity-0 cursor-pointer w-full h-full"
                  title="Click to pick color"
                />
              </div>

              <div className="flex-1 space-y-2 font-sans">
                <div className="text-xl font-bold text-gray-100 font-mono tracking-wider">
                  {colorData.hex.toUpperCase()}
                </div>
                <div className="text-xs text-gray-400 font-mono">
                  {colorData.rgb}
                </div>
                <div className="text-xs text-gray-400 font-mono">
                  {colorData.hsl}
                </div>
              </div>
            </div>

            {/* Formats Grid */}
            <div className="grid grid-cols-2 gap-2 font-sans">
              {[
                { label: 'HEX', val: colorData.hex },
                { label: 'RGB', val: colorData.rgb },
                { label: 'HSL', val: colorData.hsl },
                { label: 'Tailwind BG', val: colorData.tailwindBg },
              ].map((fmt) => (
                <button
                  key={fmt.label}
                  onClick={() => onCopyFormat(fmt.label, fmt.val)}
                  className="flex items-center justify-between px-3 py-2 rounded-xl bg-[#1d202c] hover:bg-[#272b3c] border border-gray-800 hover:border-gray-700 text-xs transition-all group"
                >
                  <div className="flex flex-col text-left truncate pr-2">
                    <span className="text-[10px] text-gray-400 uppercase font-semibold">{fmt.label}</span>
                    <span className="font-mono text-gray-200 truncate text-[11px]">{fmt.val}</span>
                  </div>
                  {copiedFormat === fmt.label ? (
                    <Check className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                  ) : (
                    <Copy className="w-3.5 h-3.5 text-gray-500 group-hover:text-gray-200 shrink-0 transition-colors" />
                  )}
                </button>
              ))}
            </div>

            {/* Contrast Ratio Preview */}
            <div className="pt-2 border-t border-gray-800/80 flex items-center justify-between text-xs font-sans">
              <div
                className="px-3 py-1.5 rounded-lg font-semibold flex items-center space-x-1.5 border border-white/10"
                style={{ backgroundColor: colorData.hex, color: '#ffffff' }}
              >
                <span>White Text</span>
                <span className="text-[10px] opacity-80">({colorData.contrastWithWhite}:1)</span>
              </div>
              <div
                className="px-3 py-1.5 rounded-lg font-semibold flex items-center space-x-1.5 border border-black/10"
                style={{ backgroundColor: colorData.hex, color: '#000000' }}
              >
                <span>Black Text</span>
                <span className="text-[10px] opacity-80">({colorData.contrastWithBlack}:1)</span>
              </div>
            </div>
          </div>
        ) : clip.content_type === 'image' ? (
          <div className="space-y-4 font-sans">
            <div className="flex flex-col items-center justify-center p-6 bg-[#161820] rounded-xl border border-gray-800 shadow-inner">
              {resolvedImageBase64 ? (
                <img
                  src={resolvedImageBase64}
                  alt="Full Preview"
                  className="max-h-96 object-contain rounded-lg shadow-2xl"
                />
              ) : (
                <div className="flex items-center space-x-2 text-gray-400 py-12">
                  <Sparkles className="w-5 h-5 animate-spin text-cyan-400" />
                  <span>Loading image preview...</span>
                </div>
              )}
            </div>

            {/* Native macOS Vision OCR Card */}
            <div className="p-4 bg-[#161820] rounded-xl border border-gray-800 space-y-3 shadow-lg">
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-2 text-cyan-400 font-semibold text-xs">
                  <ScanText className="w-4 h-4" />
                  <span>Extracted Image Text (macOS Vision OCR)</span>
                </div>

                <div className="flex items-center space-x-1.5">
                  {clip.text_content && (
                    <button
                      onClick={() => onCopyFormat('OCR Text', clip.text_content || '')}
                      className="p-1.5 rounded-lg bg-cyan-950/80 hover:bg-cyan-900 border border-cyan-700/60 text-cyan-300 transition-all cursor-pointer"
                      title={copiedFormat === 'OCR Text' ? 'Copied!' : 'Copy Extracted Text'}
                    >
                      <Copy className="w-3.5 h-3.5" />
                    </button>
                  )}
                  <button
                    onClick={onRunOCR}
                    disabled={isOcrLoading || readOnly}
                    className="p-1.5 rounded-lg bg-white hover:bg-gray-200 text-black transition-all shadow cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed disabled:hover:bg-white"
                    title={readOnly ? 'Restore this clip to run OCR' : isOcrLoading ? 'Extracting...' : clip.text_content ? 'Re-Run OCR' : 'Extract Text (OCR)'}
                  >
                    <Sparkles className={`w-3.5 h-3.5 text-cyan-600 ${isOcrLoading ? 'animate-spin' : ''}`} />
                  </button>
                </div>
              </div>

              {clip.text_content ? (
                <div className="p-3.5 bg-[#0f1118] border border-gray-800/80 rounded-xl text-gray-200 font-mono text-xs whitespace-pre-wrap leading-relaxed select-text shadow-inner max-h-60 overflow-y-auto">
                  {clip.text_content}
                </div>
              ) : (
                <p className="text-xs text-gray-400 italic">
                  Click "Extract Text (OCR)" above or copy screenshot to run native macOS Vision text recognition.
                </p>
              )}
            </div>
          </div>
        ) : (
          <div className="p-4 bg-[#171717] rounded-xl border border-[#2f2f2f] text-gray-200 leading-relaxed overflow-x-auto whitespace-pre-wrap selection:bg-gray-700 selection:text-white shadow-inner">
            {displayText}
          </div>
        )}
    </>
  );
}
