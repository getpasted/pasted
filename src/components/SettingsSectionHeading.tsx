interface SettingsSectionHeadingProps {
  title: string;
  description?: string;
  align?: 'left' | 'center';
  className?: string;
}

export function SettingsSectionHeading({
  title,
  description,
  align = 'left',
  className = '',
}: SettingsSectionHeadingProps) {
  return (
    <div className={`${align === 'center' ? 'text-center' : 'text-start'} ${className}`.trim()}>
      <h3 className="theme-text-muted text-[10px] font-semibold uppercase tracking-wider">
        {title}
      </h3>
      {description && (
        <p className="theme-text-muted mt-1 text-[11px] leading-normal">
          {description}
        </p>
      )}
    </div>
  );
}
