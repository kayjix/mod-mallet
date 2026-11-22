// Which patch the decisions go to (ADR 0070), shown at the sidebar's foot on every
// page where something is decided, with a quick switch to another of the
// user's own patches or to a new one.

import { useSession } from "./Actions";
import { activePatch, isForeign, withNewPatch } from "./patch-book";
import { patchTexts } from "./patch-texts";
import "./editing-chip.css";

/** The value of the choice that starts a new patch. */
const NEW = "\u0000new";

/** "Decisions go to: Patch X ▾", with the patch page one click away. */
export function EditingChip() {
  const { book, setBook, lang, openPatch } = useSession();
  const p = patchTexts[lang];
  const edited = book === null ? undefined : activePatch(book);
  if (book === null || edited === undefined) {
    return null;
  }
  const own = book.patches.filter((patch) => !isForeign(patch));
  return (
    <div className="editing-chip side-card" title={p.chipHint}>
      <button
        type="button"
        className="editing-chip-label"
        onClick={() =>
          openPatch({ patch: edited.id, entry: null, label: p.patchLabel(edited.name) })
        }
      >
        {p.chipLabel}
      </button>
      <select
        aria-label={p.chipLabel}
        value={edited.id}
        onChange={(event) => {
          const id = event.target.value;
          if (id === NEW) {
            setBook(withNewPatch(book, p.newPatchName).book);
          } else {
            setBook({ ...book, active: id });
          }
        }}
      >
        {own.map((patch) => (
          <option key={patch.id} value={patch.id}>
            {patch.name}
            {patch.off === true ? ` (${p.state["off-here"]})` : ""}
          </option>
        ))}
        <option value={NEW}>+ {p.newPatch}</option>
      </select>
    </div>
  );
}
