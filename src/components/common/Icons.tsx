// Decorative inline icons (stroke style). Every icon is hidden from screen readers.
import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement> & { size?: number };

function base({ size = 16, ...rest }: IconProps) {
  return { width: size, height: size, "aria-hidden": true as const, focusable: false as const, ...rest };
}

export function AppIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 16 16" {...base(props)}>
      <defs>
        <linearGradient id="fd-app-icon" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#ff5e3a" />
          <stop offset="1" stopColor="#8e24aa" />
        </linearGradient>
      </defs>
      <rect width="16" height="16" rx="4" fill="url(#fd-app-icon)" />
      <path d="M8.6 3.5L5.5 8.6h2.3l-1 3.9 3.6-5.3H8.1z" fill="#ffffff" />
    </svg>
  );
}

export function FolderIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 20 20" {...base({ size: 20, ...props })}>
      <path
        d="M2 5.5A1.5 1.5 0 013.5 4h4l1.6 1.8h7.4A1.5 1.5 0 0118 7.3v7.2a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5z"
        fill="var(--folder)"
      />
    </svg>
  );
}

export function LinkFolderIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 20 20" {...base({ size: 20, ...props })}>
      <path
        d="M2 5.5A1.5 1.5 0 013.5 4h4l1.6 1.8h7.4A1.5 1.5 0 0118 7.3v7.2a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 14.5z"
        fill="var(--folder)"
        opacity="0.55"
      />
      <path d="M8 13.5l4-4M9.5 9.5H12V12" fill="none" stroke="var(--text)" strokeWidth="1.4" strokeLinecap="round" />
    </svg>
  );
}

export function DriveIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 20 20" {...base({ size: 20, ...props })}>
      <rect x="2" y="5" width="16" height="10" rx="2" fill="none" stroke="var(--text-2)" strokeWidth="1.4" />
      <circle cx="14.5" cy="10" r="1" fill="var(--accent)" />
    </svg>
  );
}

export function PlusIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 12 12" {...base({ size: 12, ...props })}>
      <path d="M6 1.5v9M1.5 6h9" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
    </svg>
  );
}

export function CheckIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 12 12" {...base({ size: 12, ...props })}>
      <path d="M2 6.5l2.5 2.5L10 3.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function CloseIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 12 12" {...base({ size: 12, ...props })}>
      <path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
    </svg>
  );
}

export function ChevronRightIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 10 10" {...base({ size: 10, ...props })}>
      <path d="M3.5 2l3 3-3 3" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function ArrowUpIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 16 16" {...base(props)}>
      <path d="M8 13V3M3.5 7.5L8 3l4.5 4.5" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function RefreshIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 16 16" {...base(props)}>
      <path
        d="M13 8a5 5 0 11-1.46-3.54M13 2.5v3h-3"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function DropIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 18 18" {...base({ size: 18, ...props })}>
      <path d="M9 2v9M5.5 7.5L9 11l3.5-3.5M3 13v2h12v-2" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function SuccessIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 18 18" {...base({ size: 18, ...props })}>
      <circle cx="9" cy="9" r="8" fill="var(--ok)" />
      <path d="M5.5 9.3l2.3 2.2 4.7-4.8" fill="none" stroke="var(--solid-bg)" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function WarningIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 18 18" {...base({ size: 18, ...props })}>
      <path d="M9 2l7.5 13h-15z" fill="var(--warn)" />
      <path d="M9 7v3.5M9 12.6v.1" stroke="var(--solid-bg)" strokeWidth="1.6" strokeLinecap="round" />
    </svg>
  );
}

export function ErrorIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 18 18" {...base({ size: 18, ...props })}>
      <circle cx="9" cy="9" r="8" fill="var(--danger-fill)" />
      <path d="M6.5 6.5l5 5M11.5 6.5l-5 5" stroke="#ffffff" strokeWidth="1.6" strokeLinecap="round" />
    </svg>
  );
}
