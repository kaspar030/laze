# Object Sharing

_Object Sharing_ describes a feature of _laze_ where identically compiled objects
are shared between builds.

Imagine both of "birthday_party" and "xmas_party" use the module "sound_system",
and are both compiled for the same builder.

When rendering a build plan, laze would place the objects of "sound_system" in a
folder like `build/out/<builder>/<app>/src/sound_system`.
But if laze can figure out that the objects are actually built identically, that
would cause them to get built multiple times.
Instead, laze places objects in `build/objects/<source_path>/filename.<hash>.o`,
where `hash` represents the hash of the ninja build statement used to build the
file. Laze also ensures that it doesn't add duplicate build statements.

This scheme effectively and automatically makes laze apps share common objects
without the need for user action.

This has shown to reduce the number of built objects for the RIOT OS by ~30-40%.
