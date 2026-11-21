// What the window shows for a game drawn with Unreal Engine (Oblivion
// Remastered, ADR 0065): a plain sentence instead of the pages that do not
// apply, and the game's Unreal mods.

import { useSession } from "./Actions";
import type { UnrealPak } from "./generated/rendered-report";
import { type NotDrawnPage, unrealTexts } from "./unreal-texts";
import "./unreal.css";

/** Says in a sentence why `page` has nothing to show for this game. */
export function NotDrawnHere({ page }: { page: NotDrawnPage }) {
  const { lang } = useSession();
  const u = unrealTexts[lang];
  return (
    <section className="panel unreal-not-drawn">
      <p>
        <strong>{u.notDrawn[page]}</strong>
      </p>
      <p className="aside">{u.why}</p>
    </section>
  );
}

/** The packages of the game's Unreal Engine side, each with its mod. */
export function UnrealPaks({ paks }: { paks: UnrealPak[] }) {
  const { lang } = useSession();
  const u = unrealTexts[lang];
  if (paks.length === 0) {
    return null;
  }
  return (
    <section className="panel unreal-paks">
      <div className="panel-head">
        <h2>{u.paksTitle}</h2>
        <span className="aside">{u.paksNote}</span>
      </div>
      <ul className="unreal-pak-list">
        {paks.map((pak) => (
          <li key={pak.path}>
            <span className="unreal-pak-name" title={pak.path}>
              {pak.path.split("/").pop()}
            </span>
            <span className="unreal-pak-folder">{u.folder[pak.folder]}</span>
            <span className="unreal-pak-mod">{pak.provider?.name ?? u.noMod}</span>
            <span className="aside">
              {pak.extensions.map((extension) => `.${extension}`).join(" ")} · {u.size(pak.bytes)}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
