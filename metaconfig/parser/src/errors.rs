/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! Definition of errors used in this crate by the error_chain crate

use failure_ext::failure_derive::Fail;
pub use failure_ext::{Error, Result};
use mononoke_types::RepositoryId;

/// Types of errors we can raise
#[derive(Debug, Fail)]
pub enum ErrorKind {
    /// The given bookmark does not exist in the repo
    #[fail(display = "bookmark not found: {}", _0)]
    BookmarkNotFound(String),
    /// The structure of metaconfig repo is invalid
    #[fail(display = "invalid file structure: {}", _0)]
    InvalidFileStructure(String),
    /// Config is invalid
    #[fail(display = "invalid config options: {}", _0)]
    InvalidConfig(String),
    /// Duplicated repo ids
    #[fail(display = "repoid {} used more than once", _0)]
    DuplicatedRepoId(RepositoryId),
    /// Missing path for hook
    #[fail(display = "missing path")]
    MissingPath(),
    /// Invalid pushvar
    #[fail(display = "invalid pushvar, should be KEY=VALUE: {}", _0)]
    InvalidPushvar(String),
    /// Too many bypass options for a hook
    #[fail(display = "Only one bypass option is allowed. Hook: {}", _0)]
    TooManyBypassOptions(String),
}
