interface IconProps {
  name: "folder" | "branch" | "refresh" | "file" | "arrow" | "settings";
}

const paths: Record<IconProps["name"], string> = {
  folder: "M3 6h6l2 2h10v10H3V6Z",
  branch: "M7 4v10a4 4 0 0 0 4 4h6M7 4m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M17 18m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M17 6m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M17 8v4a2 2 0 0 1-2 2h-4",
  refresh: "M20 7v5h-5M4 17a8 8 0 0 0 14.5 1M4 17v-5h5M20 7A8 8 0 0 0 5.5 6",
  file: "M6 3h8l4 4v14H6V3Zm8 0v5h5",
  arrow: "m9 18 6-6-6-6",
  settings: "M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Zm7.4-3.5a7.7 7.7 0 0 0-.1-1l2-1.6-2-3.4-2.4 1a8 8 0 0 0-1.8-1L14.8 3h-4L10.4 6a8 8 0 0 0-1.8 1L6.2 6 4.2 9.4l2 1.6a7.7 7.7 0 0 0 0 2L4.2 14.6l2 3.4 2.4-1a8 8 0 0 0 1.8 1l.4 3h4l.4-3a8 8 0 0 0 1.8-1l2.4 1 2-3.4-2-1.6a7.7 7.7 0 0 0 0-1Z",
};

export function Icon({ name }: IconProps) {
  return (
    <svg aria-hidden="true" className="icon" viewBox="0 0 24 24">
      <path d={paths[name]} />
    </svg>
  );
}

