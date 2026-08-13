interface StatusDotProps {
  online: boolean;
  label: string;
}

export function StatusDot({ online, label }: StatusDotProps) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 14 14"
      className="h-4 w-4"
      aria-label={label}
      role="img"
    >
      <title>{label}</title>
      <circle
        cx="7"
        cy="7"
        r="5"
        stroke="currentColor"
        strokeWidth="1"
        className="text-indigo-200 dark:text-white"
        fill={online ? "#3bd672" : "#F87171"}
      />
    </svg>
  );
}
