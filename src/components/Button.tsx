import { cn } from '@/lib/cn';

type Variant = 'default' | 'accent' | 'danger' | 'subtle' | 'ghost';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
}

const base =
  'inline-flex items-center justify-center gap-1.5 rounded-[4px] px-3 py-1.5 text-[13px] ' +
  'transition-colors select-none disabled:opacity-40 disabled:cursor-not-allowed ' +
  'focus-visible:outline-2 focus-visible:outline-offset-[-2px]';

export function Button({ variant = 'default', className, ...rest }: ButtonProps) {
  return (
    <button
      type="button"
      className={cn(base, variantClass(variant), className)}
      style={variantStyle(variant)}
      {...rest}
    />
  );
}

function variantClass(variant: Variant): string {
  switch (variant) {
    case 'accent':
      return 'text-white hover:brightness-110 active:brightness-95';
    case 'danger':
      return 'text-white hover:brightness-110 active:brightness-95';
    case 'subtle':
      return 'hover:bg-black/5 dark:hover:bg-white/10';
    case 'ghost':
      return 'hover:bg-black/5 dark:hover:bg-white/10 px-2';
    default:
      return 'hover:bg-black/5 dark:hover:bg-white/10';
  }
}

function variantStyle(variant: Variant): React.CSSProperties {
  switch (variant) {
    case 'accent':
      return { background: 'var(--win-accent)' };
    case 'danger':
      return { background: 'var(--win-danger)' };
    case 'subtle':
      return { border: '1px solid var(--win-border-strong)', background: 'var(--win-surface)' };
    case 'ghost':
      return { background: 'transparent' };
    default:
      return { border: '1px solid var(--win-border-strong)', background: 'var(--win-surface)' };
  }
}
