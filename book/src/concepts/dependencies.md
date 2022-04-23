### Module Dependencies

Apps/modules can depend on other modules. There are multiple depencency types:

1. "depends" -> a hard dependency.

   This module won't work unless the dependency is usable (not disabled, not
   conflicted by another module and its own dependencies can be resolved).
   It is pulled in and the dependency's exported environment will be imported.

   "depends" is equivalent to specifying both "selects" "uses".

2. "selects" -> also a hard dependency, like 1.), the dependency will be pulled in,
   but the dependency's exported environment will _not_ be imported.

3. "uses" -> this module is affected by the dependency.

   The dependency will not be pulled in by this module, but if it is part of the
   build through other means (e.g., another module depends on it), this module
   will import the dependency's exported environment.

4. "conflicts" -> the modules cannot be used together in the same build.

5. "optionally depends" -> a soft dependency

   If the dependency is usable (not disabled, not conflicted by another module
   and its own dependencies can be resolved), it will be pulled in and its
   exported environment will be imported.

6. "if this than that" -> if "this" is part of the dependency tree, "depend" on "that"

7. "optional if this than that" -> same as 6.) but as soft dependency.
