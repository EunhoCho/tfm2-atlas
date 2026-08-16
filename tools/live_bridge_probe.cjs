const path = require("node:path");

const { BridgeClient } = require(path.join(__dirname, "..", "desktop", "dashboard", "src", "bridge.cjs"));

async function main() {
  const command = process.argv[2] || "GET_DASHBOARD";
  const payload = process.argv[3] ? JSON.parse(process.argv[3]) : {};
  const data = await new BridgeClient({ timeoutMs: 15000 }).request(command, payload);
  if (command !== "GET_DASHBOARD") {
    process.stdout.write(`${JSON.stringify(data, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${JSON.stringify({
    connected: data.connected,
    engine_status: data.engine_status,
    career_id: data.career_id,
    player_team_id: data.player_team_id,
    data_revision: data.data_revision,
    index_progress: data.index_progress,
    champion_count: data.champion_info?.length || 0,
    tier_application: data.tier_application,
  }, null, 2)}\n`);
}

main().catch((error) => {
  process.stderr.write(`${error.code || error.message}\n`);
  process.exitCode = 1;
});
