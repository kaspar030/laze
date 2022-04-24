# `laze`

A laze import allows using laze files that are bundled inside the laze binary.

`laze` needs the name of the desired file.

Currently, the following are defined:

| Name       | Purpose                                                  |
| ---------- | -------------------------------------------------------- |
| `defaults` | a default context and host builder for simple C projects |

Example:

```yaml
imports:
  - laze: defaults
```
