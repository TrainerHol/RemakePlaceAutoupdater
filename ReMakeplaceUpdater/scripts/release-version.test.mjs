import assert from "node:assert/strict";
import test from "node:test";

import { parseReleaseVersion } from "./release-version.mjs";

test("accepts stable SemVer and returns the updater release tag", () => {
  assert.deepEqual(parseReleaseVersion("1.3.0"), {
    version: "1.3.0",
    tag: "remakeplace-updater-v1.3.0",
  });
});

test("trims whitespace from the version input", () => {
  assert.equal(parseReleaseVersion(" 2.0.1\n").version, "2.0.1");
});

test("rejects leading v, prereleases, build metadata, and malformed versions", () => {
  for (const version of ["v1.3.0", "1.3", "1.3.0-beta.1", "1.3.0+5", "01.2.3"]) {
    assert.throws(() => parseReleaseVersion(version));
  }
});
