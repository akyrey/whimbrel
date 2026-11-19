import { useLayout } from "./layoutStore";

/** Moves the response below or beside the request. */
export function PlacementToggle() {
  const split = useLayout((s) => s.split);
  const beside = split === "beside";
  const label = beside
    ? "Put the response below the request"
    : "Put the response beside the request";
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className="ml-auto pb-1 text-base leading-none text-zinc-400 hover:text-zinc-100"
      onClick={() => {
        useLayout.getState().placeResponse(beside ? "below" : "beside");
      }}
    >
      {beside ? "⬓" : "◨"}
    </button>
  );
}
