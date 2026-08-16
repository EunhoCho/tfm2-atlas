function defaultProfile() {
  return {
    enabled: true,
    scope: "solo_and_tournament",
    region: "all",
    division: "all",
    role: "all",
    patch: "latest",
    sample: { mode: "auto" },
    preset: "classic",
  };
}

function normalizeProfile(input = {}) {
  const profile = { ...defaultProfile(), ...input };
  if (profile.scope !== "tournament") profile.division = "all";
  if (!profile.patch) profile.patch = "latest";
  if (!profile.sample || !["auto", "minimum"].includes(profile.sample.mode)) {
    profile.sample = { mode: "auto" };
  }
  if (profile.sample.mode === "minimum") {
    const games = Math.max(1, Math.min(10000, Number(profile.sample.games) || 1));
    profile.sample = { mode: "minimum", games };
  }
  return profile;
}

module.exports = { defaultProfile, normalizeProfile };
