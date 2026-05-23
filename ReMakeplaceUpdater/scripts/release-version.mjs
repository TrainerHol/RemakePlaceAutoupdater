import { appendFileSync } from "node:fs";

const STABLE_SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

export function parseReleaseVersion(input) {
  const version = String(input ?? "").trim();

  if (version.startsWith("v")) {
    throw new Error("Version must not start with v. Use 1.3.0, not v1.3.0.");
  }

  if (!STABLE_SEMVER.test(version)) {
    throw new Error("Version must be stable SemVer in the form 1.3.0.");
  }

  return {
    version,
    tag: `remakeplace-updater-v${version}`,
  };
}

export function writeGitHubOutput(output) {
  const outputPath = process.env.GITHUB_OUTPUT;
  if (!outputPath) return;

  appendFileSync(outputPath, `version=${output.version}\n`);
  appendFileSync(outputPath, `tag=${output.tag}\n`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    const output = parseReleaseVersion(process.argv[2]);
    writeGitHubOutput(output);
    console.log(`${output.tag}`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
