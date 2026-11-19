---
id: doc-1
title: 'Prior art: outline storage and node addressing'
type: other
created_date: '2026-11-19 16:56'
updated_date: '2026-11-19 16:56'
---
No surveyed tool keeps a node reference stable across outside edits without either writing an ID into the source file
(Logseq file graphs, Obsidian, Org-mode) or owning the text outright (Roam, Tana, Logseq DB graphs, Unison). The tools
that may not touch the document (Hypothesis, GitHub, Gerrit) store several redundant selectors, re-anchor with fuzzy
matching, and keep failed references as "orphaned" or "outdated" instead of dropping them; this is the model that fits
Ardent's anchor.

Claims marked **inference** are my reading of the cited source, not a statement the source makes.

## Summary table

| Tool                    | Tree storage                                         | Node identity                                       | Reference survives edits?                                                                   |
| ----------------------- | ---------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Roam Research           | Hosted entity database, Datalog queries              | Block `uid`                                         | Yes for text edits and moves inside Roam; breaks on delete. No outside edits exist.        |
| Logseq file graph       | Markdown/Org files, parsed into DataScript           | UUID, persisted only as `id::` line in the file     | Only if the `id::` line stays with its block. Without it, a new UUID on every parse.        |
| Logseq DB graph         | `db.sqlite`                                          | UUID; references stored as `[[uuid]]`               | Yes, including renames. No outside file edits exist.                                        |
| Tana                    | Hosted; one owner per node                           | `nodeID`                                            | Yes inside Tana; references are mirrors with the same ID.                                   |
| tree-sitter             | In-memory syntax tree with byte and point ranges     | None stable; `Node::id` only while a node is reused | Ranges shift with `Tree::edit` if you know the edit. No identity across a fresh parse.      |
| GumTree                 | Two ASTs in memory                                   | None; computes old-to-new node mappings             | Mappings follow moves and renames between any two versions.                                 |
| difftastic              | Simplified tree of atoms and lists                   | None; position ignored                              | Display only; order-sensitive, no move detection.                                           |
| Unison                  | Codebase database keyed by AST hash                  | Hash of the definition; names are metadata          | Renames: yes. Body edits: new hash, name repointed, dependents updated.                     |
| W3C model, Hypothesis   | Annotation stored outside the document               | Redundant selectors: range, position, quote         | Fuzzy re-anchoring; failures become orphans.                                                |
| GitHub, Gerrit reviews  | Comment stored with commit, file, line or range      | (commit, path, line or range)                       | Mapped forward when possible; else "outdated" (GitHub) or moved to file level (Gerrit).     |
| SCIP, LSIF              | Index per document                                   | Symbol string: package plus descriptor path         | Survives line shifts and body edits; changes on rename or new parent (inference).           |
| Org-mode `org-id`       | Org file; `:ID:` property; rebuildable location cache | UUID by default                                     | Survives moves, also across files. Writes into the source file.                             |
| Obsidian block IDs      | Markdown; `^id` appended to the block                | Short ID chosen by user or picker                   | Survives edits that keep the marker. Writes into the source file.                           |

## Roam Research

Roam is closed source. Its developer documentation lives inside the Roam web app ([developer docs][roam-devdocs]) and
did not load as a static page. The claims below come from Roam's own SDK repository.

**Storage.** The backend API offers a Datalog query endpoint `q` and a `pull` endpoint. The SDK README queries blocks by
the attributes `:block/uid` and `:block/string` ([SDK README][roam-readme]). **Inference:** Roam stores the outline as
entities with attributes in a Datomic-style database, not as files. I found no first-party schema for the JSON or EDN
export.

**Node identity.** Every block has a `uid`. All write actions address a block by `uid`, and `create-block` may supply
its own `uid` ([Python client][roam-py]). The SDK README links documentation pages by 9-character IDs such as
`tIaOPdXCj` ([SDK README][roam-readme]). **Inference:** pages and blocks share one short, random UID space.

**References across edits.** `update-block` replaces the block `string` and keeps the `uid`. `move-block` takes the
`uid` and a new location, given as `parent-uid` plus `order` ([Python client][roam-py]). So a reference by UID survives
text edits, reordering, and reparenting by construction. It breaks on `delete-block`. Roam owns the text, so the case
"edited outside the tool" does not exist. I could not verify from a primary source what the `((uid))` reference shows
after the target is deleted.

Sources: [Roam SDK README][roam-readme], [Roam Python client][roam-py], [Roam developer docs (app only)][roam-devdocs].

## Logseq

**Storage.** A file graph stores pages as Markdown or Org files. The app parses them into DataScript, which the README
lists as a dependency ([README][lsq-readme]). A DB graph keeps "all your graph's data" in `db.sqlite`, and file graphs
now live in a separate repository, `logseq/og` ([DB changes][lsq-db-changes]). DB graphs export to SQLite, to EDN (the
only export that "fully captures a graph's data"), or to Markdown without block properties ([DB export][lsq-db]).

**Node identity.** Each block has a UUID. In a file graph, the parser reads the block's `id` property. If the block has
none, the parser generates a fresh `squuid` on every parse ([parser][lsq-parse-id]). Logseq writes `id:: <uuid>` into
the file only when a user copies a block reference: `copy-block-ref!` calls `set-blocks-id!`, documented as "Persist
block uuid to file if the uuid is valid, and it's not persisted in file" ([editor, 0.10.9][lsq-set-id]). Logseq's own
docs graph contains such lines ([docs page source][lsq-docs-ref]).

**References across edits.** A `((uuid))` reference resolves through the `id::` line. It survives any outside edit that
keeps that line under its block: lines inserted above, body edits, moves in a text editor. It breaks when the line is
deleted or detached. When an outside copy-paste duplicates an `id::` line, the parser gives the duplicate a new UUID and
strips its `id` property ([parser][lsq-dup-id]). **Inference:** parse order decides which copy keeps the old ID. In DB
graphs, block text stores references as `[[uuid]]` and renders the title ([content.cljs][lsq-content]), so renames do
not break references.

Sources: [README][lsq-readme], [DB changes][lsq-db-changes], [DB export][lsq-db], [parser][lsq-parse-id],
[duplicate-ID repair][lsq-dup-id], [`set-blocks-id!`][lsq-set-id], [docs page source][lsq-docs-ref],
[DB content refs][lsq-content].

## Tana

Tana is hosted and closed source. The claims below come from Tana's own docs.

**Storage.** "In Tana Outliner, everything is a node", including fields, views, and commands. "Every node has an owner,
a single source of truth" ([nodes][tana-nodes]). The full export, Tana JSON, "contains Tana's full stored representation
for technical use" and has no published schema ([export][tana-export]). Tana Intermediate Format (TIF) 2.1 is a
documented JSON subset that Tana can import ([TIF][tana-tif]).

**Node identity.** Every node has a `nodeID`. A user gets it from the URL that `Copy link` produces. The Input API
addresses the target node, supertags, and fields by ID ([Input API][tana-input]). In TIF, a `uid` is a label local to
one file, used for references inside that file; an `id` points to an existing node; imported definitions get fresh IDs
([TIF][tana-tif]).

**References across edits.** A reference shares the ID of the original: "Editing one reference edits the original and
thus every other reference." Deleting a reference leaves the original. Trashing the original leaves its references with
a restore icon ([nodes][tana-nodes]). References survive edits, moves, and renames because they hold the ID, not the
text. The Input API is write-only ([Input API][tana-input]). A local MCP server reads a node and its children as
Markdown ([Local API][tana-local]).

**Note for Ardent.** TIF separates file-local labels from global IDs. That split could let `.ardent/` YAML refer to
nodes before they have stable IDs.

Sources: [nodes and references][tana-nodes], [Input API][tana-input], [TIF][tana-tif], [export][tana-export],
[Local API][tana-local].

## tree-sitter

**Storage.** An in-memory syntax tree. Each node has a kind, a byte range, and row/column points ([`Node`][ts-node]).
Nothing persists.

**Node identity.** None that is stable. `Node::id` is unique within one tree. A node that incremental parsing reuses
keeps its id in the new tree, but "A node not marked as having changes does not guarantee it was reused"
([`Node::id`][ts-node-id]).

**References across edits.** The caller describes each text edit as an `InputEdit` in bytes and points, calls
`Tree::edit`, then parses again with the old tree. Stored `Node` values need `Node::edit` with the same edit
([editing][ts-editing]). `Tree::changed_ranges` compares the edited old tree with the new tree and returns "ranges whose
syntactic structure has changed" ([`changed_ranges`][ts-changed]). All of this needs the edit sequence. A file changed
by git or a formatter gives Ardent only old and new text. **Inference:** Ardent must diff the texts or the trees itself,
or parse from scratch.

**WebAssembly grammars.** `tree-sitter build --wasm` compiles a grammar to `.wasm`, and web-tree-sitter loads it with
`Language.load` ([web-tree-sitter][ts-web]). The Rust crate's `wasm` feature adds `WasmStore::new(&wasmtime::Engine)`,
`WasmStore::load_language(name, bytes)`, and `Parser::set_wasm_store` ([wasm_language.rs][ts-wasm]).
`Parser::set_language` rejects a grammar outside the supported ABI version range ([`set_language`][ts-setlang]).
**Inference:** Ardent can load format grammars at run time without native code, but this path ties the host to wasmtime.

Sources: [advanced parsing: editing][ts-editing], [`Node` docs][ts-node], [`Node::id`][ts-node-id],
[`Tree::changed_ranges`][ts-changed], [web-tree-sitter README][ts-web], [wasm_language.rs][ts-wasm],
[`Parser::set_language`][ts-setlang].

## GumTree

**Storage.** GumTree stores nothing. It takes two ASTs and returns node mappings and an edit script
([paper][gumtree-paper], [repository][gumtree-repo]).

**Node identity.** None persists. A mapping pairs one node of the old tree with one node of the new tree. A node belongs
to at most one mapping, and mapped nodes have the same label ([paper, §2][gumtree-paper]).

**References across edits.** The matcher runs in two phases ([paper, §3][gumtree-paper]):

1. Greedy top-down: find the largest isomorphic subtrees, compared by hash, above a height `minHeight`. These give
   "anchors mappings".
2. Bottom-up: match two nodes (a "container mapping") when their descendants share many anchors, measured by a Dice
   coefficient above `minDice`. Then run an optimal algorithm on small subtrees (below `maxSize`) to find "recovery
   mappings".

An algorithm by Chawathe et al. turns the mappings into an edit script of update, add, delete, and move actions. The
shortest edit script with moves is NP-hard, so GumTree uses heuristics. One evaluation setup used `minHeight = 2`,
`minDice = 0.5`, `maxSize = 100` ([paper, §5][gumtree-paper]). The README says GumTree "can detect moved or renamed
elements" ([repository][gumtree-repo]).

**Inference:** this is the re-anchoring step Ardent needs after an outside edit. Match the stored outline to the new
outline, carry each view entry along its node's mapping, and report unmapped nodes as deleted.

Sources: [Falleri et al., ASE 2014][gumtree-paper], [GumTree repository][gumtree-repo].

## difftastic

**Storage.** difftastic parses with tree-sitter and converts the parse tree into a simpler tree of atoms and lists
([parsing][dft-parsing]).

**Node identity.** None. The simple tree "only stores node content and node position", and "position is ignored during
diffing" ([parsing][dft-parsing]).

**References across edits.** The diff is a shortest-path search with Dijkstra's algorithm. Each vertex pairs a position
in each tree; each step marks a left node novel, marks a right node novel, or matches both ([diffing][dft-diffing]).
difftastic "considers ordering to be meaningful everywhere" and notes that unordered tree diffing is NP-hard. It falls
back to a line diff on parse errors ([tricky cases][dft-tricky]). **Inference:** difftastic is built for display. A
function moved within a file shows as a delete plus an insert, so it fits anchor carry-over worse than GumTree.

Sources: [parsing][dft-parsing], [diffing][dft-diffing], [tricky cases][dft-tricky].

## Unison

**Storage.** Unison keeps code in "a proper database", not in text files ([big idea][unison-idea]). "Each Unison
definition is identified by a hash of its syntax tree" ([big idea][unison-idea]).

**Node identity.** The hash. Dependencies inside a definition are replaced by their hashes, and "names are just
separately stored metadata that don't affect the function's hash" ([big idea][unison-idea]).

**References across edits.** A rename changes only the name metadata, so references by hash do not break; the docs call
this "instant non-breaking renames" ([big idea][unison-idea]). A body edit makes a new definition: "Definitions never
change, but the names we give them do." Unison computes the new hash and points the name at it, then propagates the
change to transitive dependents when the types are compatible ([FAQ][unison-faq]). Otherwise it opens the dependents in
a scratch file for review ([update how-to][unison-update]).

**Inference:** a content hash identifies one version of a node, not the node over time. It detects "unchanged" cheaply
but cannot be the anchor itself, because every body edit would orphan the view entries.

Sources: [the big idea][unison-idea], [general FAQ][unison-faq], [update how-to][unison-update].

## Annotation anchoring: W3C Web Annotation and Hypothesis

**Storage.** An annotation lives outside the document and describes its target with selectors. Several selectors may
describe the same segment; a consumer "MUST pick one" if they differ ([selectors][w3c-selectors]).

**Node identity.** The target has no ID. A `TextQuoteSelector` stores `exact` plus `prefix` and `suffix`
([quote][w3c-quote]). A `TextPositionSelector` stores `start` and `end` in code points, and the spec calls it "very
brittle with regards to changes to the resource" ([position][w3c-position]). `refinedBy` applies a narrower selector to
the result of a broader one ([refinement][w3c-refine]).

**References across edits.** The Hypothesis client stores a range (XPath) selector, a position selector, and a quote
selector with 32 characters of prefix and suffix ([types.ts][hyp-types]). `anchor()` tries range, then position, then
quote. Range and position results must equal the stored `exact` text ([html.ts][hyp-html]). Quote matching tries exact
search first, then an approximate search with up to `min(256, quote.length / 2)` errors. It scores candidates with
weights quote 50, prefix 20, suffix 20, position hint 2 ([match-quote.ts][hyp-match]). An annotation that fails to
anchor becomes an orphan and shows in a separate tab; it is not deleted ([tabs.ts][hyp-tabs]). The 2013 design used
diff-match-patch ([fuzzy anchoring post][hyp-blog]); current code uses `approx-string-match`
([match-quote.ts][hyp-match]).

**Inference:** this is the closest model for an Ardent anchor. It has redundant selectors, a cheap-to-expensive order,
verification against a stored quote, a scored fuzzy fallback, and an explicit orphan state.

Sources: [W3C selectors][w3c-selectors], [quote][w3c-quote], [position][w3c-position], [refinedBy][w3c-refine],
[html.ts][hyp-html], [match-quote.ts][hyp-match], [types.ts][hyp-types], [tabs.ts][hyp-tabs], [blog][hyp-blog].

## Code review comments: GitHub and Gerrit

**Storage.** The review system stores each comment next to the code, keyed to a commit and a file.

**Node identity.** A GitHub review comment has `commit_id`, `line`, `side`, and an optional `start_line`. GitHub
deprecated the diff-relative `position` in favor of file line numbers ([REST][gh-rest], [GraphQL][gh-graphql]). A Gerrit
comment has a `line` or a `range` on a patch set ([REST, ported comments][gerrit-ported]).

**References across edits.** GitHub keeps `originalLine` and `originalCommit` next to the current `line`, and flags a
thread `isOutdated` when it "was outdated by newer changes" ([GraphQL][gh-graphql]). **Inference:** GitHub maps comments
forward while their lines are unchanged; the exact rule is not documented. Gerrit's ported-comments endpoint computes a
position in the new revision. If it cannot find the exact position it uses "the next best position", which can turn the
comment into a file comment or move it "to a different file or the patchset level". A comment on a copied file can
appear twice with the same UUID. Gerrit tells callers not to rely on the porting logic
([ported comments][gerrit-ported]).

**Note for Ardent.** Both degrade instead of dropping: range, then file, then change. Both keep the original position
for display.

Sources: [GitHub REST review comments][gh-rest], [GitHub GraphQL objects][gh-graphql],
[Gerrit ported comments][gerrit-ported].

## Symbol monikers: SCIP and LSIF

**Storage.** A SCIP index is Protobuf, organized by document. It uses strings for IDs, not integers, because strings
help debugging and limit "the blast radius of indexer bugs" ([design][scip-design]).

**Node identity.** A symbol is `<scheme> <manager> <package-name> <version> <descriptor>+`, or `local <id>`. Descriptor
suffixes mark the kind: namespace `/`, type `#`, term `.`, method `name(disambiguator).`, type parameter `[name]`,
parameter `(name)`, meta `:`, macro `!`. Descriptors "should together form a fully qualified name", with "one descriptor
for every node in the AST" on the path ([scip.proto][scip-proto]). An example constructed from that grammar:
`rust-analyzer cargo ardent-core 0.1.0 outline/Node#parse().`. LSIF monikers have a scheme and an identifier that "must
be position independent and stable", plus a uniqueness level from `document` to `global` ([LSIF][lsif]).

**References across edits.** **Inference** from the grammar: a symbol survives inserted lines, body edits, and
reformatting. It changes on rename, on a move to another parent, and on a package version bump, because the version is
part of the string. Overloads need the method disambiguator. Local symbols are unique only within one document.

**Note for Ardent.** The descriptor grammar is a ready model for human-readable node paths: kind suffixes, escaping
rules, and a disambiguator for duplicate names.

Sources: [scip.proto][scip-proto], [SCIP design][scip-design], [LSIF 0.6.0 monikers][lsif].

## Org-mode `org-id` and Obsidian block IDs

**Storage.** Both keep plain-text source and write the ID into it. Org stores it "in the entry as an :ID: property"
([org-id.el][org-id]). Obsidian appends `^id` to a paragraph line, or puts it on its own line after a list, quote, or
table ([links][obsidian]).

**Node identity.** Org uses a UUID by default, or a time-based ID with a prefix ([org-id.el][org-id]). A human-readable
`CUSTOM_ID` may sit next to it ([manual][org-manual]). Obsidian IDs use letters, digits, and dashes, and a user may
choose a readable one ([links][obsidian]).

**References across edits.** An Org ID link "works even if the entry is moved from file to file" ([manual][org-manual]).
Org finds IDs by scanning files and caching their locations in `.org-id-locations`, which it can rebuild
([org-id.el][org-id-loc]). Obsidian updates links when a file is renamed; its docs do not say what happens when a block
or heading changes ([links][obsidian]). Both survive any edit that keeps the marker.

**Trade-off for Ardent.** Ardent's README says of a source file "Ardent reads it; the user owns it" and "A view never
changes the source file." Writing IDs into source files is therefore out, unless a user opts in. Org's split still
transfers: the ID is the truth, and the location index is a cache that a scan can rebuild.

Sources: [org-id.el][org-id], [Org manual: handling links][org-manual], [Obsidian links][obsidian].

## Ideas to adopt

- **Redundant selectors with ordered fallback** — informs TASK-2. Store several selectors per view entry: node path,
  byte range, node content hash, and quote with context. Try exact matches first, verify against the hash or quote, then
  fall back to scored fuzzy matching, as Hypothesis does.
- **An explicit orphan state** — informs TASK-2 and TASK-4. Hypothesis orphans, GitHub "outdated" threads, and Gerrit's
  file-level fallback all keep the comment. Ardent should keep the view entry and its last known position.
- **Node paths modeled on SCIP descriptors** — informs TASK-2 and TASK-5. Kind suffixes, escaping, and a disambiguator
  give readable, diff-friendly paths. Leave out the package version so paths survive releases.
- **Tree matching to resync after outside edits** — informs TASK-4. Parse the new file, match the stored structure to
  the new outline with GumTree-style top-down then bottom-up matching, and carry anchors along the mappings. Mappings
  also reveal renames (update) and moves.
- **Separate identity, name, and content** — informs TASK-2 and TASK-5. Unison keeps names apart from hashes; Logseq DB
  graphs and Tana store the ID in a reference and render the title. Ardent can give each node an opaque ID in the
  structure, and keep path and content hash as attributes.
- **Source file authoritative, structure rebuildable** — informs TASK-4. Like `.org-id-locations`, the structure should
  be a cache that a full scan can rebuild. Only view data and node IDs need care on rebuild.
- **File-local labels next to global IDs** — informs TASK-5. TIF's `uid`/`id` split lets a document refer to nodes
  before they have stable IDs.
- **tree-sitter grammars as WebAssembly** — informs TASK-3. The Rust crate's `wasm` feature loads grammar `.wasm` files
  at run time. Check the grammar ABI version on load.

## Ideas to avoid

- **Writing IDs into source files** (Logseq `id::`, Obsidian `^id`, Org `:ID:`) — informs TASK-2. It breaks the README's
  promise that views never change source files. Copy-paste also duplicates IDs, which Logseq must repair.
- **IDs regenerated on every parse** (Logseq file graph blocks without `id::`) — informs TASK-2 and TASK-5. Any ID in
  `.ardent/` must persist and be matched on resync, not minted again.
- **Offsets or line numbers as the only anchor** — informs TASK-2. The W3C spec calls position selectors "very brittle",
  and GitHub deprecated diff positions.
- **A content hash as the anchor** — informs TASK-2. Every body edit would orphan the entry, because each edit makes a
  new hash, as Unison shows.
- **tree-sitter `Node::id` or `changed_ranges` across outside edits** — informs TASK-4. Both need a known edit sequence,
  and node reuse is not guaranteed.
- **Order-sensitive diffs without move detection** (difftastic) for re-anchoring — informs TASK-4. A moved function
  would read as delete plus insert and orphan its views.
- **An undocumented storage format** (Tana JSON, Roam export) — informs TASK-5. Publish the YAML schema and put a
  version field in every file.

## Open questions this survey did not settle

- How `.ardent/` merges in git when two branches re-anchor the same file. No surveyed tool commits derived structure
  next to the source (TASK-4, TASK-5).
- What fuzzy-match thresholds suit code. Hypothesis weights target prose (TASK-2).
- Whether GumTree-style matching is fast enough for large files in a TUI, and whether a maintained Rust implementation
  exists. I did not check (TASK-4).
- How to treat a node that splits in two or two nodes that merge (TASK-2).
- Whether to persist node IDs at all, or derive identity from path plus hash on each scan (TASK-2, TASK-5).
- Whether format plugins and view plugins share one WebAssembly runtime, given that tree-sitter's Rust path uses
  wasmtime (TASK-3).
- Roam and Tana storage internals, export schemas, and Roam's behavior after a referenced block is deleted. No primary
  source was available.

[roam-devdocs]: https://roamresearch.com/#/app/developer-documentation/page/tIaOPdXCj
[roam-readme]: https://github.com/Roam-Research/backend-sdks/blob/1372e332fdf45ef2317cec17baad999eed2fa4ed/typescript/README.md
[roam-py]: https://github.com/Roam-Research/backend-sdks/blob/1372e332fdf45ef2317cec17baad999eed2fa4ed/python/roam_client/client.py#L68-L100
[lsq-readme]: https://github.com/logseq/logseq/blob/master/README.md
[lsq-db-changes]: https://github.com/logseq/docs/blob/master/db-version-changes.md
[lsq-db]: https://github.com/logseq/docs/blob/master/db-version.md#graph-export
[lsq-parse-id]: https://github.com/logseq/logseq/blob/2a0dce924d548005d8e9215b3d4467f6a07964eb/deps/graph-parser/src/logseq/graph_parser/block.cljs#L595-L603
[lsq-dup-id]: https://github.com/logseq/logseq/blob/2a0dce924d548005d8e9215b3d4467f6a07964eb/deps/graph-parser/src/logseq/graph_parser/block.cljs#L759-L786
[lsq-set-id]: https://github.com/logseq/logseq/blob/0.10.9/src/main/frontend/handler/editor.cljs#L954-L972
[lsq-docs-ref]: https://github.com/logseq/docs/blob/master/pages/The%20basics%20of%20block%20references.md
[lsq-content]: https://github.com/logseq/logseq/blob/1ded683f513259c6622c159f0ff0fb19b5c1e9fb/deps/db/src/logseq/db/frontend/content.cljs#L10-L17
[tana-nodes]: https://outliner.tana.inc/learn/features/nodes-and-references
[tana-input]: https://outliner.tana.inc/learn/features/input-api
[tana-tif]: https://outliner.tana.inc/learn/features/tana-intermediate-format
[tana-export]: https://outliner.tana.inc/learn/features/copy-paste-and-export
[tana-local]: https://outliner.tana.inc/learn/features/local-api-mcp
[ts-editing]: https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html#editing
[ts-node]: https://docs.rs/tree-sitter/0.27.2/tree_sitter/struct.Node.html
[ts-node-id]: https://github.com/tree-sitter/tree-sitter/blob/f0090e112482d66653b43ca73e85668db1cde794/lib/binding_rust/lib.rs#L1641-L1654
[ts-changed]: https://github.com/tree-sitter/tree-sitter/blob/f0090e112482d66653b43ca73e85668db1cde794/lib/binding_rust/lib.rs#L1554-L1564
[ts-setlang]: https://github.com/tree-sitter/tree-sitter/blob/f0090e112482d66653b43ca73e85668db1cde794/lib/binding_rust/lib.rs#L784-L800
[ts-wasm]: https://github.com/tree-sitter/tree-sitter/blob/f0090e112482d66653b43ca73e85668db1cde794/lib/binding_rust/wasm_language.rs#L47-L125
[ts-web]: https://github.com/tree-sitter/tree-sitter/blob/master/lib/binding_web/README.md
[gumtree-paper]: https://www.labri.fr/perso/xblanc/data/papers/ASE14.pdf
[gumtree-repo]: https://github.com/GumTreeDiff/gumtree
[dft-parsing]: https://difftastic.wilfred.me.uk/parsing.html
[dft-diffing]: https://difftastic.wilfred.me.uk/diffing.html
[dft-tricky]: https://difftastic.wilfred.me.uk/tricky_cases.html
[unison-idea]: https://www.unison-lang.org/docs/the-big-idea/
[unison-faq]: https://www.unison-lang.org/docs/usage-topics/general-faqs/
[unison-update]: https://www.unison-lang.org/docs/usage-topics/workflow-how-tos/update-code/
[w3c-selectors]: https://www.w3.org/TR/annotation-model/#selectors
[w3c-quote]: https://www.w3.org/TR/annotation-model/#text-quote-selector
[w3c-position]: https://www.w3.org/TR/annotation-model/#text-position-selector
[w3c-refine]: https://www.w3.org/TR/annotation-model/#refinement-of-selection
[hyp-html]: https://github.com/hypothesis/client/blob/72526e9c8a671ec32cb3fe8c78f7e0082ab8016d/src/annotator/anchoring/html.ts#L36-L118
[hyp-match]: https://github.com/hypothesis/client/blob/72526e9c8a671ec32cb3fe8c78f7e0082ab8016d/src/annotator/anchoring/match-quote.ts#L81-L163
[hyp-types]: https://github.com/hypothesis/client/blob/72526e9c8a671ec32cb3fe8c78f7e0082ab8016d/src/annotator/anchoring/types.ts#L199-L215
[hyp-tabs]: https://github.com/hypothesis/client/blob/72526e9c8a671ec32cb3fe8c78f7e0082ab8016d/src/sidebar/helpers/tabs.ts#L9-L17
[hyp-blog]: https://web.hypothes.is/blog/fuzzy-anchoring/
[gh-rest]: https://docs.github.com/en/rest/pulls/comments
[gh-graphql]: https://docs.github.com/en/graphql/reference/objects#pullrequestreviewthread
[gerrit-ported]: https://gerrit-review.googlesource.com/Documentation/rest-api-changes.html#list-ported-comments
[scip-proto]: https://github.com/sourcegraph/scip/blob/be1d9f769a02a11be3b3ec5485231e2531a78a1c/scip.proto
[scip-design]: https://github.com/sourcegraph/scip/blob/main/docs/DESIGN.md
[lsif]: https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/#exportsImports
[org-id]: https://github.com/emacs-mirror/emacs/blob/3b1094943be27a129f27508675fae577f9433ef3/lisp/org/org-id.el#L26-L41
[org-id-loc]: https://github.com/emacs-mirror/emacs/blob/3b1094943be27a129f27508675fae577f9433ef3/lisp/org/org-id.el#L230-L247
[org-manual]: https://orgmode.org/manual/Handling-Links.html
[obsidian]: https://obsidian.md/help/links
