import {
  forwardRef,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type CSSProperties,
  type HTMLAttributes,
  type ReactNode,
  type RefObject,
} from 'react';
import { createPortal } from 'react-dom';
import { ChevronRight } from 'lucide-react';
import { OverflowText } from './OverflowText';
import { useLocalization } from '../localization/LocalizationProvider';
import { normalizeMenuDividers } from '../utils/menuChildren';

const VIEWPORT_PADDING = 8;
const MENU_SURFACE_CLASSES = 'surface-scroll-region rounded-xl border p-1.5 text-xs font-medium select-none';

export type MenuAnchor =
  | { kind: 'point'; x: number; y: number }
  | {
      kind: 'element';
      ref: RefObject<HTMLElement | null>;
      align?: 'start' | 'end';
      gap?: number;
    };

interface AnchoredMenuProps {
  anchor: MenuAnchor;
  ariaLabel: string;
  children: ReactNode;
  className?: string;
  onClose: () => void;
  restoreFocus?: boolean;
  style?: CSSProperties;
}

interface MenuPosition {
  anchorKey: string | null;
  left: number;
  top: number;
  ready: boolean;
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(Math.max(value, minimum), Math.max(minimum, maximum));
}

/**
 * Shared portal, viewport positioning, and dismissal boundary for floating menus.
 * Embedded menus such as the Sidebar search helper intentionally remain in flow.
 */
export function AnchoredMenu({
  anchor,
  ariaLabel,
  children,
  className = '',
  onClose,
  restoreFocus = true,
  style,
}: AnchoredMenuProps) {
  const { direction } = useLocalization();
  const menuRef = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  const positionedAnchorRef = useRef<string | null>(null);
  const revealFrameRef = useRef<number | null>(null);
  const scrollIdleTimerRef = useRef<number | null>(null);
  onCloseRef.current = onClose;
  const anchorKind = anchor.kind;
  const anchorX = anchor.kind === 'point' ? anchor.x : undefined;
  const anchorY = anchor.kind === 'point' ? anchor.y : undefined;
  const anchorRef = anchor.kind === 'element' ? anchor.ref : undefined;
  const anchorAlign = anchor.kind === 'element' ? anchor.align : undefined;
  const anchorGap = anchor.kind === 'element' ? anchor.gap : undefined;
  const anchorKey = anchor.kind === 'point'
    ? `point:${anchor.x}:${anchor.y}`
    : `element:${anchor.align ?? 'end'}:${anchor.gap ?? 6}:${direction}`;
  const [position, setPosition] = useState<MenuPosition>({
    anchorKey: null,
    left: VIEWPORT_PADDING,
    top: VIEWPORT_PADDING,
    ready: false,
  });
  const revealScrollbar = (element: HTMLDivElement) => {
    element.classList.add('is-scrolling');
    if (scrollIdleTimerRef.current !== null) window.clearTimeout(scrollIdleTimerRef.current);
    scrollIdleTimerRef.current = window.setTimeout(() => {
      element.classList.remove('is-scrolling');
      scrollIdleTimerRef.current = null;
    }, 900);
  };

  useLayoutEffect(() => {
    const positionMenu = () => {
      const menu = menuRef.current;
      if (!menu) return;

      const menuRect = menu.getBoundingClientRect();
      let preferredLeft: number;
      let preferredTop: number;

      if (anchor.kind === 'point') {
        preferredLeft = anchor.x;
        preferredTop = anchor.y;
      } else {
        const anchorElement = anchor.ref.current;
        if (!anchorElement) {
          onCloseRef.current();
          return;
        }
        const anchorRect = anchorElement.getBoundingClientRect();
        const gap = anchor.gap ?? 6;
        const alignToPhysicalLeft = (anchor.align === 'start') === (direction === 'ltr');
        preferredLeft = alignToPhysicalLeft ? anchorRect.left : anchorRect.right - menuRect.width;
        const fitsBelow = anchorRect.bottom + gap + menuRect.height <= window.innerHeight - VIEWPORT_PADDING;
        preferredTop = fitsBelow
          ? anchorRect.bottom + gap
          : anchorRect.top - gap - menuRect.height;
      }

      const needsHiddenPositioningPass = positionedAnchorRef.current !== anchorKey;
      if (revealFrameRef.current !== null) cancelAnimationFrame(revealFrameRef.current);
      setPosition({
        anchorKey,
        left: clamp(preferredLeft, VIEWPORT_PADDING, window.innerWidth - menuRect.width - VIEWPORT_PADDING),
        top: clamp(preferredTop, VIEWPORT_PADDING, window.innerHeight - menuRect.height - VIEWPORT_PADDING),
        ready: !needsHiddenPositioningPass,
      });
      positionedAnchorRef.current = anchorKey;
      if (needsHiddenPositioningPass) {
        revealFrameRef.current = requestAnimationFrame(() => {
          revealFrameRef.current = null;
          setPosition((current) => current.anchorKey === anchorKey
            ? { ...current, ready: true }
            : current);
        });
      }
    };

    positionMenu();
    window.addEventListener('resize', positionMenu);
    window.addEventListener('scroll', positionMenu, true);
    return () => {
      if (revealFrameRef.current !== null) cancelAnimationFrame(revealFrameRef.current);
      if (scrollIdleTimerRef.current !== null) window.clearTimeout(scrollIdleTimerRef.current);
      window.removeEventListener('resize', positionMenu);
      window.removeEventListener('scroll', positionMenu, true);
    };
  }, [anchorAlign, anchorGap, anchorKey, anchorKind, anchorRef, anchorX, anchorY, direction]);

  useEffect(() => {
    const closeOutside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (menuRef.current?.contains(target)) return;
      if (anchor.kind === 'element' && anchor.ref.current?.contains(target)) return;
      onClose();
    };
    const closeWithKeyboard = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      event.stopPropagation();
      onClose();
      if (restoreFocus && anchor.kind === 'element') anchor.ref.current?.focus();
    };

    // Capture before drag/reorder surfaces intentionally stop pointer events.
    window.addEventListener('pointerdown', closeOutside, true);
    window.addEventListener('keydown', closeWithKeyboard, true);
    return () => {
      window.removeEventListener('pointerdown', closeOutside, true);
      window.removeEventListener('keydown', closeWithKeyboard, true);
    };
  }, [anchorAlign, anchorGap, anchorKind, anchorRef, anchorX, anchorY, onClose, restoreFocus]);

  const isPositionReady = position.ready && position.anchorKey === anchorKey;
  const normalizedChildren = normalizeMenuDividers(children, MenuDivider);

  if (!isPositionReady) {
    return createPortal(
      <div
        key={`measure:${anchorKey}`}
        ref={menuRef}
        aria-hidden="true"
        data-anchored-menu-measurement
        className={`${MENU_SURFACE_CLASSES} fixed ${className}`}
        style={{
          ...style,
          left: -10_000,
          top: -10_000,
          visibility: 'hidden',
          pointerEvents: 'none',
        }}
      >
        {normalizedChildren}
      </div>,
      document.body,
    );
  }

  return createPortal(
    <div
      key={`visible:${anchorKey}`}
      ref={menuRef}
      role="menu"
      aria-label={ariaLabel}
      data-anchored-menu
      className={`theme-menu ${MENU_SURFACE_CLASSES} fixed animate-in fade-in zoom-in-95 duration-100 ${className}`}
      style={{
        ...style,
        left: position.left,
        top: position.top,
      }}
      onPointerDown={(event) => event.stopPropagation()}
      onClick={(event) => event.stopPropagation()}
      onWheelCapture={(event) => revealScrollbar(event.currentTarget)}
      onScroll={(event) => revealScrollbar(event.currentTarget)}
    >
      {normalizedChildren}
    </div>,
    document.body,
  );
}

interface EmbeddedMenuProps extends Omit<HTMLAttributes<HTMLDivElement>, 'aria-label'> {
  ariaLabel: string;
}

export const EmbeddedMenu = forwardRef<HTMLDivElement, EmbeddedMenuProps>(function EmbeddedMenu(
  { ariaLabel, children, className = '', ...props },
  ref,
) {
  return (
    <div
      ref={ref}
      role="menu"
      aria-label={ariaLabel}
      data-embedded-menu
      className={`theme-menu ${MENU_SURFACE_CLASSES} ${className}`}
      {...props}
    >
      {normalizeMenuDividers(children, MenuDivider)}
    </div>
  );
});

interface MenuItemProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  active?: boolean;
  danger?: boolean;
}

export const MenuItem = forwardRef<HTMLButtonElement, MenuItemProps>(function MenuItem(
  { active = false, danger = false, className = '', type = 'button', ...props },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      role={props.role ?? 'menuitem'}
      className={`theme-menu-item flex w-full items-center rounded-lg text-start disabled:cursor-not-allowed disabled:opacity-40 ${active ? 'is-selected' : ''} ${danger ? 'theme-danger-text' : ''} ${className}`}
      {...props}
    />
  );
});

export function MenuDivider({ className = '' }: { className?: string }) {
  return <div role="separator" className={`theme-menu-divider my-1 border-t ${className}`} />;
}

interface MenuSubmenuProps {
  children: ReactNode;
  icon?: ReactNode;
  label: string;
  onOpenChange: (open: boolean) => void;
  onSelect?: () => void;
  open: boolean;
  panelClassName?: string;
  triggerClassName?: string;
}

export function MenuSubmenu({
  children,
  icon,
  label,
  onOpenChange,
  onSelect,
  open,
  panelClassName = 'w-48',
  triggerClassName = '',
}: MenuSubmenuProps) {
  const { direction } = useLocalization();
  const rootRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [panelStyle, setPanelStyle] = useState<CSSProperties>(
    direction === 'rtl' ? { right: 'calc(100% - 1px)', top: -4 } : { left: 'calc(100% - 1px)', top: -4 },
  );
  const normalizedChildren = normalizeMenuDividers(children, MenuDivider);

  const cancelClose = () => {
    if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    closeTimerRef.current = null;
  };
  const openMenu = () => {
    cancelClose();
    onOpenChange(true);
  };
  const scheduleClose = () => {
    cancelClose();
    closeTimerRef.current = setTimeout(() => onOpenChange(false), 80);
  };

  useLayoutEffect(() => {
    if (!open) return;
    const root = rootRef.current;
    const panel = panelRef.current;
    if (!root || !panel) return;
    const rootRect = root.getBoundingClientRect();
    const panelRect = panel.getBoundingClientRect();
    const fitsLeft = rootRect.left - panelRect.width + 1 >= VIEWPORT_PADDING;
    const fitsRight = rootRect.right + panelRect.width - 1 <= window.innerWidth - VIEWPORT_PADDING;
    const opensLeft = direction === 'rtl' ? fitsLeft || !fitsRight : !fitsRight && fitsLeft;
    const top = clamp(-4, VIEWPORT_PADDING - rootRect.top, window.innerHeight - VIEWPORT_PADDING - rootRect.top - panelRect.height);
    setPanelStyle(opensLeft ? { right: 'calc(100% - 1px)', top } : { left: 'calc(100% - 1px)', top });
  }, [direction, open]);

  useEffect(() => () => cancelClose(), []);

  return (
    <div
      ref={rootRef}
      className="relative"
      onPointerEnter={openMenu}
      onPointerLeave={scheduleClose}
      onFocus={openMenu}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) scheduleClose();
      }}
    >
      <MenuItem
        active={open}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => {
          if (onSelect) onSelect();
          else onOpenChange(!open);
        }}
        className={`justify-between gap-3 px-3 py-1.5 ${triggerClassName}`}
      >
        <span className="flex min-w-0 items-center gap-2.5">
          {icon}
          <OverflowText text={label} className="bidi-interface-align truncate" />
        </span>
        <ChevronRight className="theme-text-muted h-3.5 w-3.5 shrink-0 rtl:-scale-x-100" aria-hidden="true" />
      </MenuItem>
      {open && (
        <div
          ref={panelRef}
          role="menu"
          aria-label={label}
          className={`theme-menu surface-scroll-region absolute rounded-xl border p-1 ${panelClassName}`}
          style={panelStyle}
          onPointerEnter={cancelClose}
          onPointerLeave={scheduleClose}
        >
          {normalizedChildren}
        </div>
      )}
    </div>
  );
}
