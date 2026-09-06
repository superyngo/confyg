// What the renderer needs from whichever shell is hosting it. Kept to exactly the calls
// `render.ts` makes today — widening it speculatively would let the renderer reach back into
// shell concerns it has no business knowing about.
export interface ShellHost {
  afterRender(): void;
  showDetail(): void;
  sectionShown(title: string): void;
  closeEntryDrawer(): void;
}
