import Lake
open Lake DSL

package «lean_solve_auto»

require auto from "../../tools/lean-auto"
require duper from "duper"

@[default_target]
lean_lib «Solve» where
  globs := #[.submodules `Solve]
