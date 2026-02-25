# Instruction For Creating New `universal-sierra-compiler` Releases

1. Create a new branch.
2. Run `./scripts/release.sh MAJOR.MINOR.PATCH`.
3. Create a pull request with title `Release MAJOR.MINOR.PATCH`.
4. Merge introduced changes to master branch.
5. Manually dispatch release workflow on Github from `master` branch.
