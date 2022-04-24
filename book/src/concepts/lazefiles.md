# Build file structure

laze uses yaml files to describe a project.

Each laze project consists of a file named `laze-project.yml` in the project's
root folder, and any number of `laze.yml` files in subdirectories.

Please see the [reference](../reference/laze_yaml.md) for a detailed description
of the file format.

laze will always read `laze-project.yml` and all referenced `laze.yml` files of
a project. Don't worry, that's fairly fast, e.g., reading and parsing ~650 build
files of RIOT takes ~35ms on a Thinkpad T480s. And it's cached, if no buildfile
has been changed since it was last read, loading the cache takes less than 10ms.

Once laze has read the build files, it will _configure_ builds as requested
when executing it. This will resolve all dependencies for the requested builds,
configure the environments and write a Ninja build file.

Once done configuring, laze will automatically call Ninja with the changed build
configuration. Ninja will then do the actual building.
