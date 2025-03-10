// Copyright 2022 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::Path;

use crate::common::CommandOutput;
use crate::common::TestEnvironment;

#[test]
fn test_edit() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");
    std::fs::write(repo_path.join("file1"), "0").unwrap();
    test_env
        .run_jj_in(&repo_path, ["commit", "-m", "first"])
        .success();
    test_env
        .run_jj_in(&repo_path, ["describe", "-m", "second"])
        .success();
    std::fs::write(repo_path.join("file1"), "1").unwrap();

    // Errors out without argument
    let output = test_env.run_jj_in(&repo_path, ["edit"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    error: the following required arguments were not provided:
      <REVSET>

    Usage: jj edit <REVSET>

    For more information, try '--help'.
    [EOF]
    [exit status: 2]
    ");

    // Makes the specified commit the working-copy commit
    let output = test_env.run_jj_in(&repo_path, ["edit", "@-"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy now at: qpvuntsm 73383c0b first
    Parent commit      : zzzzzzzz 00000000 (empty) (no description set)
    Added 0 files, modified 1 files, removed 0 files
    [EOF]
    ");
    let output = get_log_output(&test_env, &repo_path);
    insta::assert_snapshot!(output, @r"
    ○  2c910ae2d628 second
    @  73383c0b6439 first
    ◆  000000000000
    [EOF]
    ");
    insta::assert_snapshot!(read_file(&repo_path.join("file1")), @"0");

    // Changes in the working copy are amended into the commit
    std::fs::write(repo_path.join("file2"), "0").unwrap();
    let output = get_log_output(&test_env, &repo_path);
    insta::assert_snapshot!(output, @r"
    ○  b384b2cc1883 second
    @  ff3f7b0dc386 first
    ◆  000000000000
    [EOF]
    ------- stderr -------
    Rebased 1 descendant commits onto updated working copy
    [EOF]
    ");
}

#[test]
// Windows says "Access is denied" when trying to delete the object file.
#[cfg(unix)]
fn test_edit_current_wc_commit_missing() {
    // Test that we get a reasonable error message when the current working-copy
    // commit is missing
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");
    test_env
        .run_jj_in(&repo_path, ["commit", "-m", "first"])
        .success();
    test_env
        .run_jj_in(&repo_path, ["describe", "-m", "second"])
        .success();
    test_env.run_jj_in(&repo_path, ["edit", "@-"]).success();

    let wc_id = test_env
        .run_jj_in(&repo_path, ["log", "--no-graph", "-T=commit_id", "-r=@"])
        .success()
        .stdout
        .into_raw();
    let wc_child_id = test_env
        .run_jj_in(&repo_path, ["log", "--no-graph", "-T=commit_id", "-r=@+"])
        .success()
        .stdout
        .into_raw();
    // Make the Git backend fail to read the current working copy commit
    let commit_object_path = repo_path
        .join(".jj")
        .join("repo")
        .join("store")
        .join("git")
        .join("objects")
        .join(&wc_id[..2])
        .join(&wc_id[2..]);
    std::fs::remove_file(commit_object_path).unwrap();

    // Pass --ignore-working-copy to avoid triggering the error at snapshot time
    let output = test_env.run_jj_in(&repo_path, ["edit", "--ignore-working-copy", &wc_child_id]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Internal error: Failed to edit a commit
    Caused by:
    1: Current working-copy commit not found
    2: Object fa15625b4a986997697639dfc2844138900c79f2 of type commit not found
    3: An object with id fa15625b4a986997697639dfc2844138900c79f2 could not be found
    [EOF]
    [exit status: 255]
    ");
}

fn read_file(path: &Path) -> String {
    String::from_utf8(std::fs::read(path).unwrap()).unwrap()
}

#[must_use]
fn get_log_output(test_env: &TestEnvironment, cwd: &Path) -> CommandOutput {
    let template = r#"commit_id.short() ++ " " ++ description"#;
    test_env.run_jj_in(cwd, ["log", "-T", template])
}
