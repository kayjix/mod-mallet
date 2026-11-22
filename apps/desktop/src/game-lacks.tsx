// A plain sentence instead of a page or a view that does not apply to the
// game looked at (ADR 0078).

import { useSession } from "./Actions";
import { gameLackTexts } from "./game-lacks-texts";
import type { GameLack } from "./generated/rendered-report";
import "./unreal.css";

/** Says in a sentence why `lack` has nothing to show for the game in use. */
export function GameLacks({ lack }: { lack: GameLack }) {
  const sentence = useGameLackSentence(lack);
  return (
    <section className="panel unreal-not-drawn">
      <p>
        <strong>{sentence}</strong>
      </p>
    </section>
  );
}

/** The sentence about `lack` for the game in use, whether or not it lacks it. */
function useGameLackSentence(lack: GameLack): string {
  const { lang, game, t } = useSession();
  const name = t.games.find((known) => known.id === game)?.name ?? game;
  return gameLackTexts[lang][lack](name);
}

/** The sentence about `lack`, or `null` if the game in use has that part. */
export function useGameLack(lack: GameLack): string | null {
  const { lacks } = useSession();
  const sentence = useGameLackSentence(lack);
  return lacks.includes(lack) ? sentence : null;
}
