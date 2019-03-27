  $ enable amend rebase histedit fbhistedit

We need obsmarkers for now, to allow unstable commits
  $ enable obsstore

  $ cat >> $HGRCPATH <<EOF
  > [mutation]
  > record=true
  > enabled=true
  > date=0 0
  > EOF

  $ . "$TESTDIR/library.sh"
  $ . "$TESTDIR/infinitepush/library.sh"
  $ cp $HGRCPATH $TESTTMP/defaulthgrc
  $ setupcommon
  $ hg init repo
  $ cd repo
  $ setupserver
  $ cd ..
  $ hg clone ssh://user@dummy/repo client -q
  $ cd client
  $ mkcommit initialcommit
  $ hg push -q -r . --create
  $ mkcommit scratchcommit

Make a scratch branch with an initial commit.
  $ hg push -q -r . --to scratch/mybranch --create

Amend the commit a couple of times and push to the scratch branch again
  $ hg amend -m "scratchcommit (amended 1)"
  $ hg amend -m "scratchcommit (amended 2)"
  $ hg push -q -r . --to scratch/mybranch --non-forward-move

Clone the repo again, and pull the scratch branch.
  $ cd ..
  $ hg clone ssh://user@dummy/repo client2 -q
  $ cd client2
  $ hg pull -q -B scratch/mybranch

Amend the commit a couple of times again.
  $ hg up tip
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ hg amend -m "scratchcommit (amended 3)"
  $ hg amend -m "scratchcommit (amended 4)"
  $ hg push -q -r . --to scratch/mybranch --non-forward-move

Pull the branch back into the original repo.
  $ cd ..
  $ cd client
  $ hg pull -B scratch/mybranch
  pulling from ssh://user@dummy/repo
  searching for changes
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 0 changes to 1 files (+1 heads)
  new changesets f1f3b31bcda8
  (run 'hg heads' to see heads, 'hg merge' to merge)
  $ hg up -q tip

We have the predecessor chain that links all versions of the commits.
This works even though we are missing the third amended version.
  $ hg log -r 'predecessors(.)' -T '{node|short} {desc}\n' --hidden
  20759b6926ce scratchcommit
  ef7d26c88be0 scratchcommit (amended 1)
  598fd30ad501 scratchcommit (amended 2)
  f1f3b31bcda8 scratchcommit (amended 4)

Something more complicated involving splits and folds.
  $ drawdag --print <<EOS
  >     E      # split: A -> C,D
  >     |      # rebase: B -> E
  >  B  D F    # fold: D, E -> F
  >  |  |/
  >  A  C
  >  |  |
  >  f1f3b
  > EOS
  f9407b1692b9 A
  80b4b0467fc6 B
  91713f37cee7 C
  113fbb421191 D
  798e89a318d8 E
  9d3e6062ef0c F
  f1f3b31bcda8 f1f3b
  $ hg up -q $F

Push commit A to a scratch branch, simulating a pushbackup.
  $ hg push --to scratch/$A -r $A --create --hidden
  pushing to ssh://user@dummy/repo
  searching for changes
  remote: pushing 2 commits:
  remote:     f1f3b31bcda8  scratchcommit (amended 4)
  remote:     f9407b1692b9  A

Push the current commit to the scratch branch.
  $ hg push --to scratch/mybranch -r .
  pushing to ssh://user@dummy/repo
  searching for changes
  remote: pushing 3 commits:
  remote:     f1f3b31bcda8  scratchcommit (amended 4)
  remote:     91713f37cee7  C
  remote:     9d3e6062ef0c  F

Pull the scratch branch and commit A into the repo.
  $ cd ..
  $ cd client2
  $ hg pull -q -B scratch/mybranch
  $ hg pull -q -r $A
  $ hg up -q $F

The predecessor information successfully reaches from F to A
  $ hg log -r "predecessors(.)" -T "{node|short} {desc}\n" --hidden
  9d3e6062ef0c F
  f9407b1692b9 A

The successor information succesfully reaches from A to C and F (it was split)
  $ hg log -r "successors($A)" -T "{node|short} {desc}\n" --hidden
  91713f37cee7 C
  9d3e6062ef0c F
  f9407b1692b9 A
