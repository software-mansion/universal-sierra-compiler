# Universal-Sierra-Compiler

Universal-Sierra-Compiler is the tool for Sierra compilation. It compiles any ever-existing Sierra version to the CASM.

## Usage

To use the tool, just pass your Sierra json file path:

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

> 📝 **Note**
>
> `casm.json` can be path to existing (or not) file.
> If the file exists its contents will be overwritten.
> 