// The catalogue of things (ADR 0053): every character, weapon, piece of
// armour and other thing of the game and the mods, alone or as the sets
// characters wear, on the same stage as the comparison of duels.

import { type ReactNode, useEffect, useMemo, useState } from "react";
import { useSession } from "./Actions";
import {
  type CatalogueTexts,
  GROUP_OF,
  type Group,
  outfitWords,
  useCatalogueTexts,
} from "./catalogue-texts";
import type {
  Catalogue,
  CatalogueItem,
  CatalogueOutfit,
  CatalogueSource,
} from "./generated/catalogue";
import { Stage, type StageItem } from "./Viewer3D";
import "./catalogue.css";

type Arrange = "category" | "material" | "outfit";

/** Where things come from, as the filter has it; `mod:` names one mod. */
type Origin = "all" | "game" | "creation-club" | "mods" | "changed" | `mod:${string}`;

type Selection = { kind: "item"; id: string } | { kind: "outfit"; id: string };

const GROUPS: Group[] = ["characters", "weapons", "apparel", "items", "other"];

/** How many rows a section of the list shows before *show more*. */
const PAGE = 150;

/** How many weapons a set puts on the stage. */
const STAGE_WEAPONS = 2;

/** What the page knows about the catalogue, looked up once. */
interface Index {
  catalogue: Catalogue;
  items: Map<string, CatalogueItem>;
  outfits: Map<string, CatalogueOutfit>;
  sources: Map<string, CatalogueSource>;
  /** The sets each thing is part of. */
  setsOf: Map<string, CatalogueOutfit[]>;
  /** The mods that bring or change things, with how many. */
  mods: [string, number][];
}

function indexOf(catalogue: Catalogue): Index {
  const items = new Map(catalogue.items.map((item) => [item.id, item]));
  const outfits = new Map(catalogue.outfits.map((outfit) => [outfit.id, outfit]));
  const sources = new Map(catalogue.sources.map((source) => [source.plugin, source]));
  const setsOf = new Map<string, CatalogueOutfit[]>();
  for (const outfit of catalogue.outfits) {
    for (const id of [...outfit.worn, ...outfit.weapons]) {
      setsOf.set(id, [...(setsOf.get(id) ?? []), outfit]);
    }
  }
  const counts = new Map<string, number>();
  for (const item of catalogue.items) {
    const mods = new Set<string>();
    for (const plugin of [item.plugin, ...(item.changedBy ?? [])]) {
      const source = sources.get(plugin);
      if (source?.origin === "mod") {
        mods.add(modName(source));
      }
    }
    for (const mod of mods) {
      counts.set(mod, (counts.get(mod) ?? 0) + 1);
    }
  }
  const mods = [...counts].sort(([a], [b]) => a.localeCompare(b));
  return { catalogue, items, outfits, sources, setsOf, mods };
}

/** The name of the mod that brings a plugin, or the plugin's own name. */
function modName(source: CatalogueSource): string {
  return source.mod ?? source.plugin;
}

function nameOf(item: CatalogueItem): string {
  return item.name ?? item.editorId ?? item.id;
}

/** The name of a set: its body piece, or else its internal name in words. */
function outfitTitle(outfit: CatalogueOutfit, index: Index): string {
  const body = outfit.worn
    .map((id) => index.items.get(id))
    .find((item) => item?.category === "body");
  if (body !== undefined) {
    return nameOf(body);
  }
  return outfit.editorId === null || outfit.editorId === undefined
    ? outfit.id
    : outfitWords(outfit.editorId);
}

/** Whether a thing comes from where the filter asks. */
function fromOrigin(item: CatalogueItem, origin: Origin, index: Index): boolean {
  const source = index.sources.get(item.plugin);
  const changers = (item.changedBy ?? []).map((plugin) => index.sources.get(plugin));
  const changedByMod = changers.some((changer) => changer?.origin === "mod");
  switch (origin) {
    case "all":
      return true;
    case "game":
      return source?.origin === "game" && !changedByMod;
    case "creation-club":
      return source?.origin === "creationClub";
    case "mods":
      return source?.origin === "mod";
    case "changed":
      return changedByMod;
    default: {
      const wanted = origin.slice("mod:".length);
      return [source, ...changers].some(
        (found) => found?.origin === "mod" && modName(found) === wanted,
      );
    }
  }
}

function matchesSearch(item: CatalogueItem, wanted: string): boolean {
  return (
    wanted === "" ||
    (item.name ?? "").toLowerCase().includes(wanted) ||
    (item.editorId ?? "").toLowerCase().includes(wanted)
  );
}

export function CatalogueView({ scannedAt }: { scannedAt: number }) {
  const { api, lang } = useSession();
  const c = useCatalogueTexts();
  const [catalogue, setCatalogue] = useState<Catalogue | null | "loading">("loading");
  const [group, setGroup] = useState<Group>("weapons");
  const [arrange, setArrange] = useState<Arrange>("category");
  const [origin, setOrigin] = useState<Origin>("all");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Selection | null>(null);

  useEffect(() => {
    let live = true;
    // The scan is named so that a new scan reads the catalogue again.
    void scannedAt;
    api.catalogue(lang).then(
      (found) => live && setCatalogue(found),
      () => live && setCatalogue(null),
    );
    return () => {
      live = false;
    };
  }, [api, lang, scannedAt]);

  const index = useMemo(
    () => (catalogue === null || catalogue === "loading" ? null : indexOf(catalogue)),
    [catalogue],
  );

  if (catalogue === "loading") {
    return <p className="aside catalogue-message">{c.loading}</p>;
  }
  if (index === null) {
    return <p className="aside catalogue-message">{c.unreadable}</p>;
  }

  // Materials say nothing about characters or the things to carry.
  const arrangements: Arrange[] =
    group === "weapons" || group === "apparel"
      ? ["category", "material", "outfit"]
      : ["category", "outfit"];
  const shownArrange = arrangements.includes(arrange) ? arrange : "category";

  return (
    <div className="catalogue">
      <aside className="catalogue-left">
        <p className="aside catalogue-intro">{c.intro}</p>
        <fieldset className="segmented catalogue-groups" aria-label={c.groupsLabel}>
          {GROUPS.map((candidate) => (
            <button
              key={candidate}
              type="button"
              aria-pressed={group === candidate}
              onClick={() => setGroup(candidate)}
            >
              {c.groups[candidate]}
            </button>
          ))}
        </fieldset>
        <div className="catalogue-filters">
          <input
            type="search"
            placeholder={c.search}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
          <label className="catalogue-field">
            <span>{c.originLabel}</span>
            <select value={origin} onChange={(event) => setOrigin(event.target.value as Origin)}>
              <option value="all">{c.originAll}</option>
              <option value="game">{c.originGame}</option>
              <option value="creation-club">{c.originCreationClub}</option>
              <option value="mods">{c.originMods}</option>
              <option value="changed">{c.originChanged}</option>
              {index.mods.length > 0 && (
                <optgroup label="Mods">
                  {index.mods.map(([mod, count]) => (
                    <option key={mod} value={`mod:${mod}`}>
                      {c.originMod(mod)} ({count})
                    </option>
                  ))}
                </optgroup>
              )}
            </select>
          </label>
          <fieldset className="segmented" aria-label={c.arrangeLabel}>
            {arrangements.map((candidate) => (
              <button
                key={candidate}
                type="button"
                aria-pressed={shownArrange === candidate}
                onClick={() => setArrange(candidate)}
              >
                {candidate === "category"
                  ? group === "characters"
                    ? c.byRace
                    : c.byCategory
                  : candidate === "material"
                    ? c.byMaterial
                    : c.byOutfit}
              </button>
            ))}
          </fieldset>
        </div>
        {shownArrange === "outfit" ? (
          <OutfitList
            index={index}
            group={group}
            origin={origin}
            search={search.trim().toLowerCase()}
            selected={selected}
            onSelect={setSelected}
          />
        ) : (
          <ItemList
            index={index}
            group={group}
            arrange={shownArrange}
            origin={origin}
            search={search.trim().toLowerCase()}
            selected={selected}
            onSelect={setSelected}
          />
        )}
      </aside>
      {selected === null ? (
        <p className="aside catalogue-message catalogue-pick">{c.pickSomething}</p>
      ) : selected.kind === "item" ? (
        <ItemView index={index} id={selected.id} onSelect={setSelected} />
      ) : (
        <OutfitView index={index} id={selected.id} onSelect={setSelected} />
      )}
    </div>
  );
}

// ---- left: the list ------------------------------------------------------------

interface ListProps {
  index: Index;
  group: Group;
  origin: Origin;
  search: string;
  selected: Selection | null;
  onSelect: (selection: Selection) => void;
}

function ItemList({
  index,
  group,
  arrange,
  origin,
  search,
  selected,
  onSelect,
}: ListProps & { arrange: "category" | "material" }) {
  const c = useCatalogueTexts();
  const sections = useMemo(() => {
    const found = new Map<string, CatalogueItem[]>();
    for (const item of index.catalogue.items) {
      if (
        GROUP_OF[item.category] !== group ||
        !matchesSearch(item, search) ||
        !fromOrigin(item, origin, index)
      ) {
        continue;
      }
      const key = sectionOf(item, arrange, c);
      found.set(key, [...(found.get(key) ?? []), item]);
    }
    return [...found].sort(([a], [b]) => a.localeCompare(b));
  }, [index, group, arrange, origin, search, c]);

  if (sections.length === 0) {
    return <p className="aside">{c.nothingFound}</p>;
  }
  const selectedId = selected?.kind === "item" ? selected.id : null;
  return (
    <ul className="catalogue-list">
      {sections.map(([title, items]) => (
        <Section
          key={title}
          title={title}
          count={items.length}
          // A search, or the thing looked at, keeps its section open.
          startOpen={search !== "" || sections.length === 1}
          holdsSelected={items.some((item) => item.id === selectedId)}
        >
          <Paged
            rows={items.map((item) => (
              <li key={item.id}>
                <ItemRow
                  item={item}
                  index={index}
                  selected={item.id === selectedId}
                  onClick={() => onSelect({ kind: "item", id: item.id })}
                />
              </li>
            ))}
          />
        </Section>
      ))}
    </ul>
  );
}

/** The heading a thing is listed under. */
function sectionOf(item: CatalogueItem, arrange: "category" | "material", c: CatalogueTexts) {
  if (item.category === "character") {
    return item.race ?? c.noRace;
  }
  if (arrange === "material") {
    return item.material === null || item.material === undefined
      ? c.noMaterial
      : c.material(item.material);
  }
  return c.categories[item.category];
}

function Section({
  title,
  count,
  startOpen,
  holdsSelected,
  children,
}: {
  title: string;
  count: number;
  startOpen: boolean;
  holdsSelected: boolean;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(startOpen);
  const shown = open || startOpen || holdsSelected;
  return (
    <li className="catalogue-section">
      <button
        type="button"
        className="catalogue-section-head"
        aria-expanded={shown}
        onClick={() => setOpen(!shown)}
      >
        <span aria-hidden="true">{shown ? "▾" : "▸"}</span>
        <span>{title}</span>
        <span className="aside">{count}</span>
      </button>
      {shown && <ul className="catalogue-rows">{children}</ul>}
    </li>
  );
}

/** Rows shown a page at a time, so that long sections stay quick. */
function Paged({ rows }: { rows: ReactNode[] }) {
  const c = useCatalogueTexts();
  const [pages, setPages] = useState(1);
  const limit = pages * PAGE;
  return (
    <>
      {rows.slice(0, limit)}
      {rows.length > limit && (
        <li>
          <button type="button" className="link-button" onClick={() => setPages(pages + 1)}>
            {c.showMore(Math.min(PAGE, rows.length - limit))}
          </button>
        </li>
      )}
    </>
  );
}

function ItemRow({
  item,
  index,
  selected,
  onClick,
}: {
  item: CatalogueItem;
  index: Index;
  selected: boolean;
  onClick: () => void;
}) {
  const source = index.sources.get(item.plugin);
  const changers = (item.changedBy ?? [])
    .map((plugin) => index.sources.get(plugin))
    .filter((found) => found?.origin === "mod");
  return (
    <button type="button" className="catalogue-row" aria-current={selected} onClick={onClick}>
      <span className="catalogue-row-name">{nameOf(item)}</span>
      <span className="catalogue-row-from">
        {source?.origin === "mod" && <span>{modName(source)}</span>}
        {changers.length > 0 && <span className="catalogue-changed">✎ {changers.length}</span>}
      </span>
    </button>
  );
}

function OutfitList({ index, group, origin, search, selected, onSelect }: ListProps) {
  const c = useCatalogueTexts();
  const outfits = useMemo(() => {
    // A set is listed with the things the page lists: under weapons, sets
    // whose wearers carry weapons; otherwise every set.
    return index.catalogue.outfits
      .filter((outfit) => group !== "weapons" || outfit.weapons.length > 0)
      .filter((outfit) => {
        const pieces = [...outfit.worn, ...outfit.weapons]
          .map((id) => index.items.get(id))
          .filter((item): item is CatalogueItem => item !== undefined);
        const searched =
          search === "" ||
          (outfit.editorId ?? "").toLowerCase().includes(search) ||
          outfitTitle(outfit, index).toLowerCase().includes(search) ||
          pieces.some((piece) => matchesSearch(piece, search));
        return (
          searched && (origin === "all" || pieces.some((piece) => fromOrigin(piece, origin, index)))
        );
      })
      .sort(
        (a, b) => b.characters - a.characters || (a.editorId ?? "").localeCompare(b.editorId ?? ""),
      );
  }, [index, group, origin, search]);

  if (outfits.length === 0) {
    return <p className="aside">{c.nothingFound}</p>;
  }
  const selectedId = selected?.kind === "outfit" ? selected.id : null;
  return (
    <ul className="catalogue-list catalogue-rows">
      <Paged
        rows={outfits.map((outfit) => (
          <li key={outfit.id}>
            <button
              type="button"
              className="catalogue-row catalogue-set-row"
              aria-current={outfit.id === selectedId}
              onClick={() => onSelect({ kind: "outfit", id: outfit.id })}
            >
              <span className="catalogue-row-name">{outfitTitle(outfit, index)}</span>
              <span className="catalogue-row-from">
                <span>{c.setPieces(outfit.worn.length, outfit.weapons.length)}</span>
                {outfit.characters > 0 && <span>{c.worn(outfit.characters)}</span>}
              </span>
              {outfit.editorId && <span className="catalogue-internal">{outfit.editorId}</span>}
            </button>
          </li>
        ))}
      />
    </ul>
  );
}

// ---- middle and right: one thing, or one set ---------------------------------------

function ItemView({
  index,
  id,
  onSelect,
}: {
  index: Index;
  id: string;
  onSelect: (selection: Selection) => void;
}) {
  const c = useCatalogueTexts();
  const item = index.items.get(id);
  if (item === undefined) {
    return <p className="aside catalogue-message catalogue-pick">{c.pickSomething}</p>;
  }
  const stage: StageItem[] =
    item.category === "character"
      ? item.models.slice(0, 1).map((path) => ({ kind: "mesh", label: c.face, path }))
      : item.models.slice(0, 2).map((path, at) => ({
          kind: "mesh",
          label: item.models.length > 1 ? (at === 0 ? c.onGround : c.asWorn) : nameOf(item),
          path,
        }));
  const outfit =
    item.outfit === null || item.outfit === undefined ? undefined : index.outfits.get(item.outfit);
  const sets = index.setsOf.get(item.id) ?? [];
  return (
    <>
      <section className="catalogue-middle">
        <h2 className="catalogue-title">{nameOf(item)}</h2>
        <Stage key={item.id} items={stage} />
      </section>
      <aside className="catalogue-right">
        <dl className="catalogue-details">
          <dt>{item.category === "character" ? c.detailRace : c.detailCategory}</dt>
          <dd>
            {item.category === "character" ? (item.race ?? c.noRace) : c.categories[item.category]}
          </dd>
          {item.material && (
            <>
              <dt>{c.detailMaterial}</dt>
              <dd>{c.material(item.material)}</dd>
            </>
          )}
          <dt>{c.detailFrom}</dt>
          <dd>
            <SourceName plugin={item.plugin} index={index} />
          </dd>
          {(item.changedBy ?? []).length > 0 && (
            <>
              <dt>{c.detailChangedBy}</dt>
              <dd>
                <ol className="catalogue-changers">
                  {(item.changedBy ?? []).map((plugin) => (
                    <li key={plugin}>
                      <SourceName plugin={plugin} index={index} />
                    </li>
                  ))}
                </ol>
              </dd>
            </>
          )}
          {item.editorId && (
            <>
              <dt>{c.detailInternal}</dt>
              <dd className="catalogue-internal">{item.editorId}</dd>
            </>
          )}
        </dl>
        {(item.variants ?? 0) > 0 && (
          <p className="aside">{c.detailVariants(item.variants ?? 0)}</p>
        )}
        {outfit !== undefined && (
          <>
            <h3>{c.detailOutfit}</h3>
            <SetLinks outfits={[outfit]} index={index} onSelect={onSelect} />
          </>
        )}
        {sets.length > 0 && (
          <>
            <h3>{c.detailInOutfits}</h3>
            <SetLinks outfits={sets} index={index} onSelect={onSelect} />
          </>
        )}
        <h3>{c.detailModels}</h3>
        <ul className="catalogue-files">
          {item.models.map((path) => (
            <li key={path}>{path}</li>
          ))}
        </ul>
      </aside>
    </>
  );
}

function OutfitView({
  index,
  id,
  onSelect,
}: {
  index: Index;
  id: string;
  onSelect: (selection: Selection) => void;
}) {
  const c = useCatalogueTexts();
  const outfit = index.outfits.get(id);
  if (outfit === undefined) {
    return <p className="aside catalogue-message catalogue-pick">{c.pickSomething}</p>;
  }
  const pieces = (ids: string[]) =>
    ids
      .map((piece) => index.items.get(piece))
      .filter((item): item is CatalogueItem => item !== undefined);
  const worn = pieces(outfit.worn);
  const weapons = pieces(outfit.weapons);
  const stage: StageItem[] = [...worn, ...weapons.slice(0, STAGE_WEAPONS)].flatMap((item) =>
    item.models.slice(0, 1).map((path) => ({ kind: "mesh" as const, label: nameOf(item), path })),
  );
  const pieceList = (items: CatalogueItem[]) => (
    <ul className="catalogue-pieces">
      {items.map((item) => (
        <li key={item.id}>
          <button
            type="button"
            className="link-button"
            onClick={() => onSelect({ kind: "item", id: item.id })}
          >
            {nameOf(item)}
          </button>
          <span className="aside"> · {c.categories[item.category]}</span>
        </li>
      ))}
    </ul>
  );
  return (
    <>
      <section className="catalogue-middle">
        <h2 className="catalogue-title">{outfitTitle(outfit, index)}</h2>
        <Stage key={outfit.id} items={stage} />
      </section>
      <aside className="catalogue-right">
        <dl className="catalogue-details">
          <dt>{c.detailFrom}</dt>
          <dd>
            <SourceName plugin={outfit.plugin} index={index} />
          </dd>
          {outfit.editorId && (
            <>
              <dt>{c.detailInternal}</dt>
              <dd className="catalogue-internal">{outfit.editorId}</dd>
            </>
          )}
        </dl>
        {outfit.characters > 0 && <p className="aside">{c.worn(outfit.characters)}</p>}
        <h3>{c.detailWorn}</h3>
        {pieceList(worn)}
        {weapons.length > 0 && (
          <>
            <h3>{c.detailWeapons}</h3>
            <p className="aside">{c.detailWeaponsHint}</p>
            {pieceList(weapons)}
          </>
        )}
      </aside>
    </>
  );
}

function SetLinks({
  outfits,
  index,
  onSelect,
}: {
  outfits: CatalogueOutfit[];
  index: Index;
  onSelect: (selection: Selection) => void;
}) {
  return (
    <ul className="catalogue-pieces">
      {outfits.map((outfit) => (
        <li key={outfit.id}>
          <button
            type="button"
            className="link-button"
            onClick={() => onSelect({ kind: "outfit", id: outfit.id })}
          >
            {outfitTitle(outfit, index)}
          </button>
          {outfit.editorId && <span className="catalogue-internal"> {outfit.editorId}</span>}
        </li>
      ))}
    </ul>
  );
}

/** Where a plugin comes from, in words: the game, the Creation Club or the mod. */
function SourceName({ plugin, index }: { plugin: string; index: Index }) {
  const c = useCatalogueTexts();
  const source = index.sources.get(plugin);
  const name =
    source?.origin === "game"
      ? c.game
      : source?.origin === "creationClub"
        ? c.creationClub
        : source === undefined
          ? plugin
          : modName(source);
  return (
    <span>
      {name} <span className="catalogue-internal">{plugin}</span>
    </span>
  );
}
