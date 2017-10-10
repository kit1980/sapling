#require p4

  $ . $TESTDIR/p4setup.sh

Populate the Depot
  $ mkdir Main
  $ echo a > Main/a
  $ echo b > Main/b
  $ p4 add Main/a Main/b
  //depot/Main/a#1 - opened for add
  //depot/Main/b#1 - opened for add
  $ p4 submit -d initial
  Submitting change 1.
  Locking 2 files ...
  add //depot/Main/a#1
  add //depot/Main/b#1
  Change 1 submitted.

  $ p4 edit Main/a
  //depot/Main/a#1 - opened for edit
  $ echo a >> Main/a
  $ p4 submit -d second
  Submitting change 2.
  Locking 1 files ...
  edit //depot/Main/a#2
  Change 2 submitted.

Fast Import

  $ cd $hgwd
  $ hg init --config 'format.usefncache=False'
  $ hg p4fastimport --bookmark master --debug -P $P4ROOT hg-p4-import
  loading changelist numbers.
  2 changelists to import.
  loading list of files.
  2 files to import.
  reading filelog * (glob)
  reading filelog * (glob)
  importing repository.
  writing filelog: b789fdd96dc2, p1 000000000000, linkrev 0, 2 bytes, src: *, path: Main/a (glob)
  writing filelog: a80d06849b33, p1 b789fdd96dc2, linkrev 1, 4 bytes, src: *, path: Main/a (glob)
  writing filelog: 1e88685f5dde, p1 000000000000, linkrev 0, 2 bytes, src: *, path: Main/b (glob)
  changelist 1: writing manifest. node: dbcf87f9f16c p1: 000000000000 p2: 000000000000 linkrev: 0
  changelist 1: writing changelog: initial
  changelist 2: writing manifest. node: 5c8695bebd8f p1: dbcf87f9f16c p2: 000000000000 linkrev: 1
  changelist 2: writing changelog: second
  writing bookmark
  updating the branch cache
  2 revision(s), 2 file(s) imported.
  $ cd $p4wd
  $ p4 edit Main/b
  //depot/Main/b#1 - opened for edit
  $ echo b >> Main/b
  $ echo c > Main/c
  $ mkdir Main/d
  $ echo e > Main/d/e
  $ mkdir Main/d/f
  $ echo g > Main/d/f/g
  $ p4 add Main/c Main/d/e Main/d/f/g
  //depot/Main/c#1 - opened for add
  //depot/Main/d/e#1 - opened for add
  //depot/Main/d/f/g#1 - opened for add
  $ p4 delete Main/a
  //depot/Main/a#2 - opened for delete
  $ p4 submit -d third
  Submitting change 3.
  Locking 5 files ...
  delete //depot/Main/a#3
  edit //depot/Main/b#2
  add //depot/Main/c#1
  add //depot/Main/d/e#1
  add //depot/Main/d/f/g#1
  Change 3 submitted.

Sync Import

  $ cd $hgwd
  $ cd $hgwd
  $ hg p4syncimport --bookmark master --debug -P $P4ROOT hg-p4-import
  incremental import from changelist: 3, node: * (glob)
  Latest change list number 3
  running a sync import.
  writing filelog: 861f64b39056, p1 1e88685f5dde, linkrev 2, 4 bytes, src: *, path: Main/b (glob)
  writing filelog: 149da44f2a4e, p1 000000000000, linkrev 2, 2 bytes, src: *, path: Main/c (glob)
  writing filelog: 6b67ccefd5ce, p1 000000000000, linkrev 2, 2 bytes, src: *, path: Main/d/e (glob)
  writing filelog: 0973eb1b2ecc, p1 000000000000, linkrev 2, 2 bytes, src: *, path: Main/d/f/g (glob)
  changelist 3: writing manifest. node: f0ca72fbd536 p1: 5c8695bebd8f p2: 000000000000 linkrev: 2
  changelist 3: writing changelog: p4fastimport synchronizing client view
  writing bookmark
  updating the branch cache
  1 revision, 4 file(s) imported.

Sync Import without New Changes

  $ hg p4syncimport --bookmark master --debug -P $P4ROOT hg-p4-import
  incremental import from changelist: 4, node: * (glob)
  Latest change list number 3
  running a sync import.
  writing filelog: 0a738da2fcb2, p1 861f64b39056, linkrev 3, 4 bytes, src: *, path: Main/b (glob)
  writing filelog: 825765709d02, p1 149da44f2a4e, linkrev 3, 2 bytes, src: *, path: Main/c (glob)
  writing filelog: c3bd1155d122, p1 6b67ccefd5ce, linkrev 3, 2 bytes, src: *, path: Main/d/e (glob)
  writing filelog: 7fd4bac0fb3a, p1 0973eb1b2ecc, linkrev 3, 2 bytes, src: *, path: Main/d/f/g (glob)
  changelist 3: writing manifest. node: c11fdd72fcbf p1: f0ca72fbd536 p2: 000000000000 linkrev: 3
  changelist 3: writing changelog: p4fastimport synchronizing client view
  writing bookmark
  updating the branch cache
  1 revision, 4 file(s) imported.

Fast Import after Sync Import

  $ hg p4fastimport --bookmark master --debug -P $P4ROOT hg-p4-import
  incremental import from changelist: 4, node: * (glob)
  loading changelist numbers.
  0 changelists to import.

Verify

  $ hg verify
  checking changesets
  checking manifests
  crosschecking files in changesets and manifests
  checking files
  5 files, 4 changesets, 11 total revisions

  $ hg update master
  4 files updated, 0 files merged, 0 files removed, 0 files unresolved
  (activating bookmark master)

  $ hg manifest -r master
  Main/b
  Main/c
  Main/d/e
  Main/d/f/g

End Test
  stopping the p4 server
