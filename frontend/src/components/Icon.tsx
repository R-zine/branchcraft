interface IconProps {
  name: "folder" | "branch" | "refresh" | "file" | "arrow";
}

const paths: Record<IconProps["name"], string> = {
  folder: "M3 6h6l2 2h10v10H3V6Z",
  branch: "M7 4v10a4 4 0 0 0 4 4h6M7 4m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M17 18m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M17 6m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M17 8v4a2 2 0 0 1-2 2h-4",
  refresh: "M20 7v5h-5M4 17a8 8 0 0 0 14.5 1M4 17v-5h5M20 7A8 8 0 0 0 5.5 6",
  file: "M6 3h8l4 4v14H6V3Zm8 0v5h5",
  arrow: "m9 18 6-6-6-6",
};

export function Icon({ name }: IconProps) {
  return (
    <svg aria-hidden="true" className="icon" viewBox="0 0 24 24">
      <path d={paths[name]} />
    </svg>
  );
}

