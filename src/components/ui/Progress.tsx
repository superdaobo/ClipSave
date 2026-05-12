import { HTMLAttributes } from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

interface ProgressProps extends HTMLAttributes<HTMLDivElement> {
  value: number;
}

export function Progress({ value, className, ...props }: ProgressProps) {
  const clampedValue = Math.min(100, Math.max(0, value));

  return (
    <div
      className={twMerge(
        clsx('w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden', className)
      )}
      role="progressbar"
      aria-valuenow={clampedValue}
      aria-valuemin={0}
      aria-valuemax={100}
      {...props}
    >
      <div
        className="h-full bg-blue-500 rounded-full transition-all duration-300"
        style={{ width: `${clampedValue}%` }}
      />
    </div>
  );
}
