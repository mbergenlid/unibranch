## Unibranch


```

            *
            |
            |
            *      * (remote_branch_head)
            |     /
            |    /
c1          *   * (local_branch_head)
            |  /
            | /
(origin)    *

```


```

            *
            |    * (Merge)
            |   / \
            *  /   * (remote_branch_head)
            | * <-/------------------------ cherry-pick c1 local_branch_head (resolve conflicts by accepting theirs)
            |  \ /
c1          *   * (local_branch_head)
            |  /
            | /
(origin)    *

```

```

            *
            |    * (Merge)
            |   / \
            *  /   * (remote_branch_head)
            | * <-/------------------------ cherry-pick c1 local_branch_head (resolve conflicts by accepting theirs)
            |  \ /
c1          *   * (local_branch_head)
            |  /
            | /
(origin)    *

```

```
                    * (Merge with 'main')
            *      /  \
            |     /    * (Merge)
            |    /    / \
c1          *   /    /   * (remote_branch_head)
            |  /    * <-/------------------------ applying "diff c1 c^" to "local_branch_head" (cherry-pick c1 local_branch_head)
            | /      \ /
(origin)    *         * (local_branch_head)
            |        /
            |       /
            *------/

```
