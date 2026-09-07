import Lake
open Lake DSL

package aesopdemo

require aesop from git
  "https://github.com/leanprover-community/aesop" @ "v4.33.0"

@[default_target]
lean_lib Aesopdemo
