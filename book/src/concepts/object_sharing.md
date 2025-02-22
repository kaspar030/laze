# Object Sharing

Object Sharing in laze

Object Sharing is a useful feature of laze that enables the sharing of identical compiled objects between builds. This means that if two different builds are using the same module and are compiled for the same builder, laze will avoid compiling the module multiple times.

When generating a build plan, laze would normally place the objects of a module in a directory like `build/out/<builder>/<app>/src/<module>`. With Object Sharing enabled, laze will instead place objects in `build/objects/<source_path>/filename.<hash>.o`, where hash represents the hash of the ninja build statement used to build the file. In addition, laze makes sure that it doesn't include duplicate build statements.

This scheme effectively and automatically makes laze apps share common objects, which eliminates the need for users to take any action to achieve this. In testing with the RIOT OS, this feature has reduced the number of built objects by approximately 30-40%.

In summary, Object Sharing in laze is a powerful feature that streamlines the build process by avoiding the redundant compilation of identical objects. This can lead to significant time and resource savings for users, particularly in large-scale projects.
