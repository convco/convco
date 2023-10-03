# Changelog

## v0.4.3 (2023-10-03)

### Features

* **commit:** intend to add and patch unstaged changes (50cdbbe), closes #146

### Fixes

* **check:** do match start of line and end of line for the default scope
regex (9986150), closes #145

### v0.4.2 (2023-08-27)

#### Fixes

* return non-zero exit code on from-stdin fail (19c1682)

### v0.4.1 (2023-07-25)

#### Features

* strip prefix regex (3f0d30d)
* make `zlib-ng-compat` a default feature flag (4b17cb6)

#### Fixes

* **check:** check should not fail if read from stdin (69ace8d), closes #130
* **check:** check should fail on unrecognized types from stdin (951253f),
closes #53
* parse issues in description (2778b81), closes #122
* **changelog:** changelog generator should respect the --prefix flag for
unreleased version (011d2ad), closes #123

## v0.4.0 (2023-02-23)

### ⚠ BREAKING CHANGE

* when not run in a tty the flag is necessary to indicateit should read from stdin



### Features

* **check:** add `--strip` flag when checking from stdin (23cd8cf), closes
#114
* **version:** allows bumping a prerelease version (e731f08), closes #90

### Fixes

* **check:** push oid from revparse single instead of calling push_ref
(a4b4111), closes #117
* read from stdin only when --from-stdin is provided (aaeb8dc)
* **commit:** explicit exit process on ctrlc (68966aa), closes #113

### v0.3.15 (2023-01-18)

#### Fixes

* **check:** read from stdin when rev is not set (7840b8a), closes #102

### v0.3.14 (2023-01-13)

#### Fixes

* only check stdin when rev is HEAD and a tty (b1bb07e), closes #102
* **changelog:** correctly set unreleased header (7bff516), closes #101

### v0.3.13 (2023-01-12)

#### Features

* **check:** check from stdin if not a tty (f53b02c), closes #100
* **changelog:** add output flag (ca37fe4)
* **changelog:** customize unreleased header (d5cc605), closes #85
* add cli parameter to show the configuration (2a8c92a), closes #95

#### Fixes

* **changelog:** correctly render last version if limited by `--max-*`
(6caa988), closes #92
* improve error handling in case the git repo could not be opened (f3003db),
closes #86
* **changelog:** improve word wrap and line length config/flags (0fa3ea4),
closes #84

### v0.3.12 (2022-09-25)

#### Features

* **check:** filter commits made by git revert (57837e8)
* **changelog:** filter max majors/minors/patches (cb508c6), closes #59
* use anyhow for error handling of commands (51abd51)

### v0.3.11 (2022-07-09)

#### Features

* **commit:** use template to format message (d408340), closes #54
* add `--first-parent` option to changelog and check command (f3b900c), closes
#60 #61 #67
* add `--merges` option for changelog and check command (2248da3), closes #60
#61 #67
* **changelog:** Support monorepo (d42285a)
* **version:** Support monorepo (f8ed823)
* **changelog:** include hidden sections on demand (618603d), closes #56

#### Fixes

* use scheme of url if available (e1d3910), closes #68
* **check:** fail for invalid commit types (d2a34a8), closes #53

### v0.3.10 (2022-04-24)

#### Fixes

* **changelog:** correctly format links (0286f48), closes #47
* set right version for unreleased repo (2d4d673), closes #49
* parse issue references correctly (#50) (b31ae66), closes #50 #48
* support gitlab subgroups at url (#44) (995a527), closes #44

### v0.3.9 (2022-03-02)

#### Fixes

* **commit:** take BREAKING CHANGE footer into account for major version
change (712f455), closes #40
* add version flag back (910781e)

### v0.3.8 (2021-12-28)

#### Features

* **commit:** improve commit command (67ecc89), closes #36
* **changelog:** limit number of tags (632c5ca), closes #35

#### Fixes

* **commit:** ensure the cursor re-appears when interrupting (2f8d9f0), closes
#33 #33

### v0.3.7 (2021-10-13)

#### Features

* **commit:** empty message in editor aborts commit (260a5d2)
* **changelog:** add skip empty flag (c3f4972), closes #30
* **check:** add max-number option (a781286), closes #18

#### Fixes

* **commit:** do not require save for editor (5945dca)
* **commit:** ensure leading and trailing whitespaces are removed from
different fields that make up a commit message (4c6734f), closes #25
* **changelog:** limit lines in changelog to default of 80 characters to
address markdownlint issue MD013 (a01e53a), closes #20
* **changelog:** updated templates to address markdownlint issues MD032 and
MD022 (1a87e67), closes #20
* **changelog:** fixes whitespace issues (91b5b72), closes #20
* correct `is_prerelease()` for SemVer (0e99770)

### v0.3.5 (2021-05-01)

### v0.3.4 (2021-03-15)

#### Features

* **check:** improve error message for invalid scope (0790327)

### v0.3.3 (2021-02-12)

#### Features

* **changelog:** make commit body available in template context (7ce4722)

#### Fixes

* Remove debug log (8c7b1fc)

### v0.3.2 (2020-10-29)

#### Features

* **commit:** improve commit dialog (dee58c2)

### v0.3.1 (2020-08-30)

#### Features

* **commit:** improve commit dialog (acf3aea)

## v0.3.0 (2020-08-23)

### ⚠ BREAKING CHANGE

* changes behaviour if `--bump` is used in combination with `--major`, `--minor` or `--patch`


### Features

* **commit:** validate commit message created by `convco commit` (76b8ff4)
* Allow a custom scope regex in the configuration (dc03118), closes #8
* **changelog:** Add option to set custom template directory in `.versionrc`
(01c9ea9), closes #3

### Fixes

* **version:** prioritize `--major` `--minor` `--patch` over `--bump`
(8c728a8)

### v0.2.3 (2020-05-17)

#### Features

* relax regex for scope to allow -/_ as separator (61ee293)
* allow a scope to contain numbers (768492a)

### v0.2.2 (2020-02-16)

#### Features

* **changelog:** find host, owner and repository from the origin url
(2675fcb)

### v0.2.1 (2020-01-21)

#### Features

* **version:** Change rules for major version zero (592c77c)

#### Fixes

* **commit:** make cli require the commit type (8c434c3)
* **changelog:** use stop revision if range is given (9bd679d)

## v0.2.0 (2020-01-12)

### Features

* **commit:** a new commit subcommand added (5c47789), closes #5

### v0.1.1 (2019-12-29)

#### Fixes

* **changelog:** take the date of the tag or last commit of a version
(bf514cd), closes #2

## v0.1.0 (2019-12-26)

### Features

* **version:** add option to print bump label (a0777ca)
* **changelog:** sort sections (fe2c9a2)
* **changelog:** parse issue references (bd7f08f)
* **changelog:** add breaking changes and read `.versionrc` file. (e521814)
* Introduces convco with 3 tools: check, version and changelog. (116ad53)
