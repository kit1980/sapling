/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::{format_err, Error};
use bookmarks::BookmarkName;
use clap::{App, Arg, ArgMatches, SubCommand};
use cmdlib::args::{self, MononokeClapApp};
use futures_ext::{try_boxfuture, BoxFuture, FutureExt};
use futures_old::future::{err, ok};
use megarepolib::common::{ChangesetArgs, ChangesetArgsFactory, StackPosition};
use mononoke_types::DateTime;

pub const BASE_COMMIT_HASH: &str = "base-commit-hash";
pub const COMMIT_HASH: &str = "commit-hash";
pub const GRADUAL_MERGE: &str = "gradual-merge";
pub const GRADUAL_MERGE_PROGRESS: &str = "gradual-merge-progress";
pub const MOVE: &str = "move";
pub const MERGE: &str = "merge";
pub const MARK_PUBLIC: &str = "mark-public";
pub const ORIGIN_REPO: &str = "origin-repo";
pub const CHANGESET: &str = "commit";
pub const FIRST_PARENT: &str = "first-parent";
pub const SECOND_PARENT: &str = "second-parent";
pub const COMMIT_MESSAGE: &str = "commit-message";
pub const COMMIT_AUTHOR: &str = "commit-author";
pub const COMMIT_DATE_RFC3339: &str = "commit-date-rfc3339";
pub const COMMIT_BOOKMARK: &str = "bookmark";
pub const DRY_RUN: &str = "dry-run";
pub const LAST_DELETION_COMMIT: &str = "last-deletion-commit";
pub const LIMIT: &str = "limit";
pub const MANUAL_COMMIT_SYNC: &str = "manual-commit-sync";
pub const PRE_DELETION_COMMIT: &str = "pre-deletion-commit";
pub const SYNC_DIAMOND_MERGE: &str = "sync-diamond-merge";
pub const MAX_NUM_OF_MOVES_IN_COMMIT: &str = "max-num-of-moves-in-commit";
pub const CHUNKING_HINT_FILE: &str = "chunking-hint-file";
pub const PARENTS: &str = "parents";
pub const PRE_MERGE_DELETE: &str = "pre-merge-delete";
pub const CATCHUP_DELETE_HEAD: &str = "create-catchup-head-deletion-commits";
pub const EVEN_CHUNK_SIZE: &str = "even-chunk-size";
pub const BONSAI_MERGE: &str = "bonsai-merge";
pub const BONSAI_MERGE_P1: &str = "bonsai-merge-p1";
pub const BONSAI_MERGE_P2: &str = "bonsai-merge-p2";
pub const HEAD_BOOKMARK: &str = "head-bookmark";
pub const TO_MERGE_CS_ID: &str = "to-merge-cs-id";
pub const PATH_REGEX: &str = "path-regex";
pub const DELETION_CHUNK_SIZE: &str = "deletion-chunk-size";
pub const WAIT_SECS: &str = "wait-secs";
pub const CATCHUP_VALIDATE_COMMAND: &str = "catchup-validate";
pub const MARK_NOT_SYNCED_COMMAND: &str = "mark-not-synced";
pub const INPUT_FILE: &str = "input-file";
pub const CHECK_PUSH_REDIRECTION_PREREQS: &str = "check-push-redirection-prereqs";
pub const VERSION: &str = "version";
pub const RUN_MOVER: &str = "run-mover";
pub const PATH: &str = "path";
pub const BACKFILL_NOOP_MAPPING: &str = "backfill-noop-mapping";
pub const MAPPING_VERSION_NAME: &str = "mapping-version-name";
pub const SOURCE_CHANGESET: &str = "source-changeset";
pub const TARGET_CHANGESET: &str = "target-changeset";

pub fn cs_args_from_matches<'a>(sub_m: &ArgMatches<'a>) -> BoxFuture<ChangesetArgs, Error> {
    let message = try_boxfuture!(
        sub_m
            .value_of(COMMIT_MESSAGE)
            .ok_or_else(|| format_err!("missing argument {}", COMMIT_MESSAGE))
    )
    .to_string();
    let author = try_boxfuture!(
        sub_m
            .value_of(COMMIT_AUTHOR)
            .ok_or_else(|| format_err!("missing argument {}", COMMIT_AUTHOR))
    )
    .to_string();
    let datetime = try_boxfuture!(
        sub_m
            .value_of(COMMIT_DATE_RFC3339)
            .map(|datetime_str| DateTime::from_rfc3339(datetime_str))
            .unwrap_or_else(|| Ok(DateTime::now()))
    );
    let bookmark = try_boxfuture!(
        sub_m
            .value_of(COMMIT_BOOKMARK)
            .map(|bookmark_str| BookmarkName::new(bookmark_str))
            .transpose()
    );
    let mark_public = sub_m.is_present(MARK_PUBLIC);
    if !mark_public && bookmark.is_some() {
        return err(format_err!(
            "--mark-public is required if --bookmark is provided"
        ))
        .boxify();
    }

    ok(ChangesetArgs {
        author,
        message,
        datetime,
        bookmark,
        mark_public,
    })
    .boxify()
}

pub fn get_delete_commits_cs_args_factory<'a>(
    sub_m: &ArgMatches<'a>,
) -> Result<Box<dyn ChangesetArgsFactory>, Error> {
    get_commit_factory(sub_m, |s, num| -> String {
        format!("[MEGAREPO DELETE] {} ({})", s, num)
    })
}

pub fn get_catchup_head_delete_commits_cs_args_factory<'a>(
    sub_m: &ArgMatches<'a>,
) -> Result<Box<dyn ChangesetArgsFactory>, Error> {
    get_commit_factory(sub_m, |s, num| -> String {
        format!("[MEGAREPO CATCHUP DELETE] {} ({})", s, num)
    })
}

pub fn get_gradual_merge_commits_cs_args_factory<'a>(
    sub_m: &ArgMatches<'a>,
) -> Result<Box<dyn ChangesetArgsFactory>, Error> {
    get_commit_factory(sub_m, |s, num| -> String {
        format!("[MEGAREPO GRADUAL MERGE] {} ({})", s, num)
    })
}

fn get_commit_factory<'a>(
    sub_m: &ArgMatches<'a>,
    msg_factory: impl Fn(&String, usize) -> String + Send + Sync + 'static,
) -> Result<Box<dyn ChangesetArgsFactory>, Error> {
    let message = sub_m
        .value_of(COMMIT_MESSAGE)
        .ok_or_else(|| format_err!("missing argument {}", COMMIT_MESSAGE))?
        .to_string();

    let author = sub_m
        .value_of(COMMIT_AUTHOR)
        .ok_or_else(|| format_err!("missing argument {}", COMMIT_AUTHOR))?
        .to_string();

    let datetime = sub_m
        .value_of(COMMIT_DATE_RFC3339)
        .map(|datetime_str| DateTime::from_rfc3339(datetime_str))
        .transpose()?
        .unwrap_or_else(|| DateTime::now());

    Ok(Box::new(move |num: StackPosition| ChangesetArgs {
        author: author.clone(),
        message: msg_factory(&message, num.0),
        datetime: datetime.clone(),
        bookmark: None,
        mark_public: false,
    }))
}

fn add_resulting_commit_args<'a, 'b>(subcommand: App<'a, 'b>) -> App<'a, 'b> {
    subcommand
        .arg(
            Arg::with_name(COMMIT_AUTHOR)
                .help("commit author to use")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(COMMIT_MESSAGE)
                .help("commit message to use")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(MARK_PUBLIC)
                .help("add the resulting commit to the public phase")
                .long(MARK_PUBLIC),
        )
        .arg(
            Arg::with_name(COMMIT_DATE_RFC3339)
                .help("commit date to use (default is now)")
                .long(COMMIT_DATE_RFC3339)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(COMMIT_BOOKMARK)
                .help("bookmark to point to resulting commits (no sanity checks, will move existing bookmark, be careful)")
                .long(COMMIT_BOOKMARK)
                .takes_value(true)
        )
}

fn add_light_resulting_commit_args<'a, 'b>(subcommand: App<'a, 'b>) -> App<'a, 'b> {
    subcommand
        .arg(
            Arg::with_name(COMMIT_AUTHOR)
                .help("commit author to use")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(COMMIT_MESSAGE)
                .help("commit message to use")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(COMMIT_DATE_RFC3339)
                .help("commit date to use (default is now)")
                .long(COMMIT_DATE_RFC3339)
                .takes_value(true),
        )
}

pub fn setup_app<'a, 'b>() -> MononokeClapApp<'a, 'b> {
    let move_subcommand = SubCommand::with_name(MOVE)
        .about("create a move commit, using a provided spec")
        .arg(
            Arg::with_name(MAX_NUM_OF_MOVES_IN_COMMIT)
                .long(MAX_NUM_OF_MOVES_IN_COMMIT)
                .help("how many files a single commit moves (note - that might create a stack of move commits instead of just one)")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name(ORIGIN_REPO)
                .help("use predefined mover for part of megarepo, coming from this repo")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(CHANGESET)
                .help("a changeset hash or bookmark of move commit's parent")
                .takes_value(true)
                .required(true),
        );

    let merge_subcommand = SubCommand::with_name(MERGE)
        .about("create a merge commit with given parents")
        .arg(
            Arg::with_name(FIRST_PARENT)
                .help("first parent of a produced merge commit")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(SECOND_PARENT)
                .help("second parent of a produced merge commit")
                .takes_value(true)
                .required(true),
        );

    let sync_diamond_subcommand = SubCommand::with_name(SYNC_DIAMOND_MERGE)
        .about("sync a diamond merge commit from a small repo into large repo")
        .arg(
            Arg::with_name(COMMIT_HASH)
                .help("diamond merge commit from small repo to sync")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(COMMIT_BOOKMARK)
                .help("bookmark to point to resulting commits (no sanity checks, will move existing bookmark, be careful)")
                .long(COMMIT_BOOKMARK)
                .takes_value(true)
        );

    let pre_merge_delete_subcommand = SubCommand::with_name(PRE_MERGE_DELETE)
        .about("create a set of pre-merge delete commtis, as well as commits to merge into the target branch")
        .arg(
            Arg::with_name(COMMIT_HASH)
                .help("commit from which to start deletion")
                .takes_value(true)
                .required(true)
        )
        .arg(
            Arg::with_name(CHUNKING_HINT_FILE)
                .help(r#"a path to working copy chunking hint. If not provided, working copy will
                        be chunked evenly into `--even-chunk-size` commits"#)
                .long(CHUNKING_HINT_FILE)
                .takes_value(true)
                .required(false)
        )
        .arg(
            Arg::with_name(EVEN_CHUNK_SIZE)
                .help("chunk size for even chunking when --chunking-hing-file is not provided")
                .long(EVEN_CHUNK_SIZE)
                .takes_value(true)
                .required(false)
        )
        .arg(
            Arg::with_name(BASE_COMMIT_HASH)
                .help("commit that will be diffed against to find what files needs to be deleted - \
                 only files that don't exist or differ from base commit will be deleted.")
                .long(BASE_COMMIT_HASH)
                .takes_value(true)
                .required(false)
        );

    let bonsai_merge_subcommand = SubCommand::with_name(BONSAI_MERGE)
        .about("create a bonsai merge commit")
        .arg(
            Arg::with_name(BONSAI_MERGE_P1)
                .help("p1 of the merge")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(BONSAI_MERGE_P2)
                .help("p2 of the merge")
                .takes_value(true)
                .required(true),
        );

    let gradual_merge_subcommand = SubCommand::with_name(GRADUAL_MERGE)
        .about("Gradually merge a list of deletion commits")
        .arg(
            Arg::with_name(LAST_DELETION_COMMIT)
                .long(LAST_DELETION_COMMIT)
                .help("Last deletion commit")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(PRE_DELETION_COMMIT)
                .long(PRE_DELETION_COMMIT)
                .help("Commit right before the first deletion commit")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(COMMIT_BOOKMARK)
                .help("bookmark to point to resulting commits (no sanity checks, will move existing bookmark, be careful)")
                .long(COMMIT_BOOKMARK)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(DRY_RUN)
                .long(DRY_RUN)
                .help("Dry-run mode - doesn't do a merge, just validates")
                .takes_value(false)
                .required(false),
        )
        .arg(
            Arg::with_name(LIMIT)
                .long(LIMIT)
                .help("how many commits to merge")
                .takes_value(true)
                .required(false),
        );

    let gradual_merge_progress_subcommand = SubCommand::with_name(GRADUAL_MERGE_PROGRESS)
        .about("Display progress of the gradual merge as #MERGED_COMMITS/#TOTAL_COMMITS_TO_MERGE")
        .arg(
            Arg::with_name(LAST_DELETION_COMMIT)
                .long(LAST_DELETION_COMMIT)
                .help("Last deletion commit")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(PRE_DELETION_COMMIT)
                .long(PRE_DELETION_COMMIT)
                .help("Commit right before the first deletion commit")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(COMMIT_BOOKMARK)
                .help("bookmark to point to resulting commits (no sanity checks, will move existing bookmark, be careful)")
                .long(COMMIT_BOOKMARK)
                .takes_value(true)
        );

    let manual_commit_sync_subcommand = SubCommand::with_name(MANUAL_COMMIT_SYNC)
        .about("Manually sync a commit from source repo to a target repo. It's usually used right after a big merge")
        .arg(
            Arg::with_name(CHANGESET)
                .long(CHANGESET)
                .help("Source repo changeset that will synced to target repo")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(DRY_RUN)
                .long(DRY_RUN)
                .help("Dry-run mode - doesn't do a merge, just validates")
                .takes_value(false)
                .required(false),
        )
        .arg(
            Arg::with_name(PARENTS)
                .long(PARENTS)
                .help("Parents of the new commit")
                .takes_value(true)
                .multiple(true)
                .required(true),
        )
        .arg(
            Arg::with_name(MAPPING_VERSION_NAME)
                .long(MAPPING_VERSION_NAME)
                .help("name of the noop mapping that will be inserted")
                .takes_value(true)
                .required(true),
        );

    let catchup_delete_head_subcommand = SubCommand::with_name(CATCHUP_DELETE_HEAD)
        .about("Create delete commits for 'catchup strategy. \
        This is normally used after invisible merge is done, but small repo got a few new commits
        that needs merging.

        O         <-  head bookmark
        |
        O   O <-  new commits (we want to merge them in master)
        |  ...
        IM  |       <- invisible merge commit
        |\\ /
        O O

        This command create deletion commits on top of master bookmark for files that were changed in new commits,
        and pushrebases them.

        After all of the commits are pushrebased paths that match --path-regex in head bookmark should be a subset
        of all paths that match --path-regex in the latest new commit we want to merge.
        ")
        .arg(
            Arg::with_name(HEAD_BOOKMARK)
                .long(HEAD_BOOKMARK)
                .help("commit to merge into")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(TO_MERGE_CS_ID)
                .long(TO_MERGE_CS_ID)
                .help("commit to merge")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(PATH_REGEX)
                .long(PATH_REGEX)
                .help("regex that matches all paths that should be merged in head commit")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(DELETION_CHUNK_SIZE)
                .long(DELETION_CHUNK_SIZE)
                .help("how many files to delete in a single commit")
                .default_value("10000")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name(WAIT_SECS)
                .long(WAIT_SECS)
                .help("how many seconds to wait after each push")
                .default_value("0")
                .takes_value(true)
                .required(false),
        );


    let catchup_validate_subcommand = SubCommand::with_name(CATCHUP_VALIDATE_COMMAND)
        .about("validate invariants about the catchup")
        .arg(
            Arg::with_name(COMMIT_HASH)
                .long(COMMIT_HASH)
                .help("merge commit i.e. commit where all catchup commits were merged into")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(TO_MERGE_CS_ID)
                .long(TO_MERGE_CS_ID)
                .help("commit to merge")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(PATH_REGEX)
                .long(PATH_REGEX)
                .help("regex that matches all paths that should be merged in head commit")
                .takes_value(true)
                .required(true),
        );

    let mark_not_synced_candidate = SubCommand::with_name(MARK_NOT_SYNCED_COMMAND)
        .about("mark all commits that do not have any mapping as not synced candidate, but leave those that have the mapping alone")
        .arg(
            Arg::with_name(INPUT_FILE)
                .long(INPUT_FILE)
                .help("list of large repo commit hashes that should be considered to be marked as not sync candidate")
                .takes_value(true)
                .required(true)
        );

    let check_push_redirection_prereqs_subcommand = SubCommand::with_name(CHECK_PUSH_REDIRECTION_PREREQS)
        .about("check the prerequisites of enabling push-redirection at a given commit with a given CommitSyncConfig version")
        .arg(
            Arg::with_name(SOURCE_CHANGESET)
                .help("a source changeset hash or bookmark to check")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(TARGET_CHANGESET)
                .help("a target changeset hash or bookmark to check")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(VERSION)
                .help("a version to use")
                .takes_value(true)
                .required(true),
        );

    let run_mover_subcommand = SubCommand::with_name(RUN_MOVER)
        .about("run mover of a given version to remap paths between source and target repos")
        .arg(
            Arg::with_name(VERSION)
                .help("a version to use")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(PATH)
                .help("a path to remap")
                .takes_value(true)
                .required(true),
        );

    let backfill_noop_mapping = SubCommand::with_name(BACKFILL_NOOP_MAPPING)
        .about(
            "
            Given the list of commit identifiers resolve them to bonsai hashes in source \
            and target repo and insert a sync commit mapping with specified version name. \
            This is useful for initial backfill to mark commits that are identical between \
            repositories. \
            Input file can contain any commit identifier (e.g. bookmark name) \
            but the safest approach is to use commit hashes (bonsai or hg).
        ",
        )
        .arg(
            Arg::with_name(INPUT_FILE)
                .long(INPUT_FILE)
                .help("list of commit hashes which are remapped with noop mapping")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(MAPPING_VERSION_NAME)
                .long(MAPPING_VERSION_NAME)
                .help("name of the noop mapping that will be inserted")
                .takes_value(true)
                .required(true),
        );

    args::MononokeAppBuilder::new("megarepo preparation tool")
        .with_advanced_args_hidden()
        .with_source_and_target_repos()
        .build()
        .subcommand(add_resulting_commit_args(move_subcommand))
        .subcommand(add_resulting_commit_args(merge_subcommand))
        .subcommand(sync_diamond_subcommand)
        .subcommand(add_light_resulting_commit_args(pre_merge_delete_subcommand))
        .subcommand(add_light_resulting_commit_args(bonsai_merge_subcommand))
        .subcommand(add_light_resulting_commit_args(gradual_merge_subcommand))
        .subcommand(gradual_merge_progress_subcommand)
        .subcommand(manual_commit_sync_subcommand)
        .subcommand(add_light_resulting_commit_args(
            catchup_delete_head_subcommand,
        ))
        .subcommand(catchup_validate_subcommand)
        .subcommand(mark_not_synced_candidate)
        .subcommand(check_push_redirection_prereqs_subcommand)
        .subcommand(run_mover_subcommand)
        .subcommand(backfill_noop_mapping)
}
