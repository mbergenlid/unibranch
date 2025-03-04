# Unibranch

Unibranch is a wrapper around git to enable a single branch workflow with stacked commits.

## Usage

Imagine you are currently on a branch called *main*

Create a new *virtual* branch from *origin/main* with the changes in the commit referenced by `<ref>`.
```
$ ubr create <ref>
```

Update the *virtual* branch for commit *ref* or all *virtual* branches if *ref* is omitted.
```
$ ubr sync [ref]
```

Push your entire working branch *origin/$USER/main*
```
$ ubr push
```

### Complete example

The idea is to always commit on main branch, so imagine that you have worked on two independent features
simultanously (Feature 1 and Feature 2). These are commited with one commit per feature straight to your
main branch, but you would now like to submit these features for review separately.

```

 (local main)  * <- Feature 2
               |
               |
               |
               * <- Feature 1
               |
               |
               |
 (origin/main) *

 ```

 ```
 $ ubr create HEAD^
 ```

 Creates a *hidden* branch for the parent commit of HEAD, i.e. first feature and pushes it to the remote.
 So now your git tree looks like this

 ```
 (local main)  * <- Feature 2
               |
               |
               |
  Feature 1 -> *   * <- origin/feature-1
               |  /
               | /
               |/
 (origin/main) *
```

You can now open a PR for *origin/feature-1* and submit it for review. In the meantime
you would also like to submit feature 2 for review.

```
$ ubr create HEAD
```

```
 (local main)  * <- Feature 2
               |
               |
               |
  Feature 1 -> *   * <- origin/feature-1
               |  /
               | /   * <- origin/feature-2
               |/   /
 (origin/main) *----
```

Imagine now that you get some feedback on you PR for *feature-1*. You address those comments and ammend your local feature-1 commit.

```
$ git commit -m "fixup! <hash-of-feature-1>"
$ git rebase origin/main --autosquash # Ammends the changes to your single feature-1 commit

$ ubr sync # Sync the local commits with the remotes 
```

Now, your tree looks like this:
```
 (local main)  * <- Feature 2
               |
               |     * <- fixup!
               |    /
  Feature 1 -> *   * <- origin/feature-1
               |  /
               | /   * <- origin/feature-2
               |/   /
 (origin/main) *----
```

Let's say feature-2 is approved and merged on the remote and after you pull the latest changes with `git pull --rebase` your local git log looks like this:
```
 (local main)  * <- Feature 1
               |
               |     * <- fixup!
               |    /
 (origin/main) *   * <- origin/feature-1
               |  /
               | /
               |/
               *
```
and after running
```
$ ubr sync
```

```
 (local main)  * <- Feature 1
               |
               |        * <- sync with main commit (essentially a merge)
               |       /
               |      /
               |     * <- fixup!
               |    /
 (origin/main) *   * <- origin/feature-1
               |  /
               | /
               |/
               *
```
