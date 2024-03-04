# Universal Sierra Compiler

The Software Mansion Team has created a new tool, Universal Sierra Compiler (further abbreviated as USC).

USC is the tool for Sierra compilation - it compiles any ever-existing Sierra version to CASM.

The idea behind USC is simple, yet it might make things easier for some tools.
It bundles all Sierra compilers previously released, and depending on the version contained inside the Sierra JSON, it chooses the right compiler.
Therefore it allows any tool to be independent of Sierra version.

Universal Sierra Compiler might become especially useful for nodes, which have to interact with contracts deployed in different versions of Sierra, but also any other tool that depends on the versioning of Sierra.


## Usage

| Command          | Purpose                               | Input                        | Input as JSON of Cairo struct                              | Output as JSON of Cairo struct |
|------------------|---------------------------------------|------------------------------|------------------------------------------------------------|-|
| compile-contract | compile Sierra of a Starknet contract | output of `cairo-compile`    | cairo_lang_starknet_classes::contract_class::ContractClass | cairo_lang_starknet::casm_contract_class::CasmContractClass |
| compile-raw      | compile Sierra of a Cairo program     | output of `starknet-compile` | cairo_lang_sierra::program::Program                        | |
|                  |                                       |                              |                                                            | |


```shell
$ universal-sierra-compiler compile-raw --sierra-path ./path/to/sierra.json

{"assembled_cairo_program": ...}
```


## Installation

Use the provided script to install the latest version of Universal Sierra Compiler:

```shell
curl -L https://raw.githubusercontent.com/software-mansion/universal-sierra-compiler/master/scripts/install.sh | sh

...
universal-sierra-compiler (v1.0.0) has been installed successfully.
```

It is also possible to install a specific version of USC by passing, for example:

```shell
| sh -s v2.0.0-rc0
```

however, since USC aims to be backwards-compatible itself, it should be enough to always use the latest version (e.g. on CI).

Note that versions of USC that are `release candidate` (rc) will not be fetched when installing the latest version. For that you would need to specify a concrete version.


## When to update USC?

We aim to prioritize creating a new release of USC available on GitHub, as soon as a new Sierra version comes out.

Only then, if you would need to use the newest Sierra, you should update the USC version.


## Why is USC a binary?

We would like USC to be as decoupled from other tools as possible. It should require the users to bump it only when a new Sierra version comes out, without the need to re-integrate it in the software that runs it.

If however, you would still prefer to use USC as a crate, please write a comment below, or [open an issue](https://github.com/software-mansion/universal-sierra-compiler/issues/new) so we can discuss it further.


---

[Check out the repo on GitHub](https://github.com/software-mansion/universal-sierra-compiler)
