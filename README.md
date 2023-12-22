# Universal-Sierra-Compiler

Universal-Sierra-Compiler is the tool for Sierra compilation. It compiles any ever-existing Sierra version to CASM.

## Usage

To use the tool, just pass a path to a file with Sierra in json format:

```shell
$ universal-sierra-compiler \
  --sierra-input-path ./path/to/sierra.json
  
{"bytecode": ...}
```

> 📝 **Note**
> 
> Please, note that the output is in the JSON format.

To automatically save CASM, pass `casm-output-path` argument:

```shell
$ universal-sierra-compiler \
  --sierra-input-path ./path/to/sierra.json
  --casm-output-path ./path/to/casm.json
```
