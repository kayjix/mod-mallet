/// <reference types="vite/client" />

// Put in by Vite when the frontend is built (ADR 0057).
/** The app's version, the same as the crates'. */
declare const __MALLET_VERSION__: string;
/** The commit the app was built from, or `null` for a development build. */
declare const __MALLET_COMMIT__: string | null;
/** The day of that commit, `YYYY-MM-DD`. */
declare const __MALLET_COMMIT_DATE__: string | null;
