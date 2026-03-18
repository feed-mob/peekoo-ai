export interface UpdateLike {
  version: string;
  date?: string;
  body?: string;
}

export interface AboutSnapshot {
  appName: string;
  currentVersion: string;
  availableVersion: string | null;
  releaseDate: string | null;
  releaseNotes: string | null;
  isUpdateAvailable: boolean;
}

interface AboutDependencies {
  getName: () => Promise<string>;
  getVersion: () => Promise<string>;
  check: () => Promise<UpdateLike | null>;
}

export async function loadAboutSnapshot(deps: AboutDependencies): Promise<{
  snapshot: AboutSnapshot;
  update: UpdateLike | null;
}> {
  const [appName, currentVersion, update] = await Promise.all([
    deps.getName(),
    deps.getVersion(),
    deps.check(),
  ]);

  return {
    snapshot: {
      appName,
      currentVersion,
      availableVersion: update?.version ?? null,
      releaseDate: update?.date ?? null,
      releaseNotes: update?.body ?? null,
      isUpdateAvailable: update !== null,
    },
    update,
  };
}
