# Universal-Sierra-Compiler

Universal-Sierra-Compiler is the tool for Sierra compilation. It compiles any ever-existing Sierra version to CASM.

| Supported Sierra Versions |
|---------------------------|
| 1.5.0                     |
| ~1.4.0                    |
| ~1.3.0                    |
| ~1.2.0                    |
| 1.1.0                     |
| 1.0.0                     |
| 0.1.0                     |

## Installation

To install the latest stable version of `universal-sierra-binary` run:

```shell
curl -L https://raw.githubusercontent.com/software-mansion/universal-sierra-compiler/master/scripts/install.sh | sh
```

You almost always want to install the latest stable version. 
In rare cases where a prerelease with a new unstable sierra version exists and you want to use it,
run the following command with the requested prerelease version:

```shell
curl -L https://raw.githubusercontent.com/software-mansion/cairo-profiler/main/scripts/install.sh | sh -s -- v0.2.0-rc.0
```

> 📝 **Note**
>
> If the script can't add installed binary to the PATH, it will print the instructions about adding it manually. 

## Usage

Tool consist of two subcommands:

- `compile-contract`
- `compile-raw`

The first one compiles Sierra of the Starknet contract, while the second one compiles Sierra of the plain Cairo code.

### `compile-contract` subcommand

The input of this subcommand is a path to a file with Sierra of the contract
(`cairo_lang_starknet_classes::contract_class::ContractClass`) in json format:

```shell
$ universal-sierra-compiler \
  compile-contract \
  --sierra-path ./path/to/sierra.json
  
{"bytecode": ...}
```

> 📝 **Note**
> 
> Please, note that the output is in the JSON format.

To automatically save CASM, pass `--output-path` argument:

```shell
$ universal-sierra-compiler \
    compile-contract \
      --sierra-path ./path/to/sierra.json
      --output-path ./path/to/casm.json
```

### `compile-raw` subcommand

The input of this subcommand is a path to a file with Sierra program (`cairo_lang_sierra::program::Program`) in json format:

```shell
$ universal-sierra-compiler \
    compile-raw \
      --sierra-path ./path/to/sierra.json
  
{"assembled_cairo_program": ...}
```

> 📝 **Note**
>
> Please, note that the output is in the JSON format.

To automatically save assebled cairo program, pass `--output-path` argument:

```shell
$ universal-sierra-compiler \
    compile-raw \
      --sierra-path ./path/to/sierra.json
      --output-path ./path/to/casm.json
```
