# lsp-gpt

... let's use GPT-4 as language server!

## what is it?

This project contains a language server (LS) implementation that uses the GPT-4 API in order
to provide suggestions according to the language server protocol (LSP) specification.
Additionally, it contains clients or usage instructions for Visual Studio Code and Neovim.

## what is still missing?

To be done before the next iteration:

- [x] explore GPT behaviour for additional LSP features: highlighting, diagnostics/errors, fixing
- [ ] enhance prompt with few-shot learning: add possibility to provide example diagrams
  - [ ] explore working on workspace level instead of document level only: other existing diagrams can be used as example
  - [ ] provide examples of requests and responses in order to shape GPT output
- [ ] decide on direction:
  - [ ] create a fully featured language server (as easily as possible)
  - [x] train it towards meaningful completions (from examples), e.g. fitting names/identifiers, desired syntax
  - [x] write a standalone language server with the help of GPT
- [ ] refactor logging


## what has been done?

The following steps have been achieved on a prototype level. Details can be found in the linked subsections.

1. utilised [prompt engineering](#prompt-engineering) in order to make the GPT-4 API act as language server
1. implemented a [language server](#language-server) that forwards LSP requests to the GPT-4 API
1. tested and verified the utilisation of the language server in [Neovim and Visual Studio Code](#usage-in-editors)

### prompt engineering

We want the GPT-4 API to act as language server: it should be able to handle requests and to respond to these requests
according to the specification of the language server protocol. Ideally these requests and responses can be directly
passed through our language server implementation.

In order to generate a fitting first prompt, we will utilise the [GPT Builder](https://chat.openai.com/gpts/editor).
This is an official tool of OpenAI that helps creating custom versions of chatGPT that excel at specific tasks.
The GPT Builder takes a short description of its desired area of expertise, like
`Make a language server that conforms to the LSP specification, and handles the plantuml file format.`, and creates
prompts that influence and customise the behaviour of chatGPT.

The short description from above resulted in the following prompt, which will directly be used later:

```
Role and Goal: lsp-gpt-plantuml is a dedicated language server for PlantUML, conforming to the Language Server Protocol (LSP). It assists with PlantUML diagrams, providing expertise in syntax, structure, and best practices.

Constraints: This GPT focuses solely on PlantUML and adheres to LSP standards. It does not support other diagram types or programming languages.

Guidelines: Responses should be clear, concise, and technically accurate. They should offer specific guidance on PlantUML syntax, diagram optimization, and best practices, all encoded in JSON format to align with LSP specifications.

Clarification: When queries are ambiguous, lsp-gpt-plantuml should request specific details about the PlantUML diagram or issue to provide the most accurate, JSON-encoded assistance.

Personalization: The GPT maintains a professional, informative tone, targeting users who seek technical assistance with PlantUML diagrams, with all responses formatted according to LSP specifications in JSON.
```

After prompting the GPT-4 API with only this text, it already works as desired. It will consume language server requests in JSON format,
and respond with valid responses that can be forwarded and parsed by the editor/language client.

Since the models used by the GPT-4 API do not have memory of previous requests, we need to include this and other possible prompts on each request sent.

For example the following 3 messages to the GPT-4 API...

```
You act as a language server that conforms to the LSP specification, and handles the plantuml file format. You are called 'lsp-gpt-plantuml'. You will be queried via API, meaning all responses should be in the JSON format.

${insert prompt from above}
```

```json
{
  "method": "textDocument/didOpen",
  "params": {
    "textDocument": {
      "languageId": "plantuml",
      "text": "@startuml example\n\nactor Alice\nactor Bob\n\nAlice -> Bob : hello\nBob --> Alice : hello Alice!\n\n@enduml\n",
      "uri": "file:///home/marco/dev/projects/lsp-gpt/assets/example.puml",
      "version": 7
    }
  }
}
```

```json
{
  "id": 5,
  "method": "textDocument/completion",
  "params": {
    "context": {
      "triggerKind": 1
    },
    "position": {
      "character": 0,
      "line": 7
    },
    "textDocument": {
      "uri": "file:///home/marco/dev/projects/lsp-gpt/assets/example.puml"
    }
  }
}
```

... result in a meaningful response:

```json
{
  "id": "chatcmpl-8XyHZ9yiVkIXX8SU4SCU3zjBgnGp5",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "{\n  \"id\": 1,\n  \"result\": [\n    {\n      \"label\": \"actor\",\n      \"kind\": 1,\n      \"detail\": \"Defines a new actor\",\n      \"documentation\": \"Use 'actor' to define a new actor in your diagram. Syntax: actor ActorName\",\n      \"insertText\": \"actor \"\n    },\n    {\n      \"label\": \"Alice\",\n      \"kind\": 1,\n      \"detail\": \"Existing actor\",\n      \"documentation\": \"Alice is an existing actor in your diagram. You can use this actor for new interactions.\",\n      \"insertText\": \"Alice\"\n    },\n    {\n      \"label\": \"Bob\",\n      \"kind\": 1,\n      \"detail\": \"Existing actor\",\n      \"documentation\": \"Bob is an existing actor in your diagram. You can use this actor for new interactions.\",\n      \"insertText\": \"Bob\"\n    },\n    {\n      \"label\": \"->\",\n      \"kind\": 1,\n      \"detail\": \"Interaction\",\n      \"documentation\": \"Use '->' to define a new interaction between two actors. Syntax: Actor1 -> Actor2 : Message\",\n      \"insertText\": \"->\"\n    },\n    {\n      \"label\": \"-->\",\n      \"kind\": 1,\n      \"detail\": \"Interaction\",\n      \"documentation\": \"Use '-->' to define a new interaction between two actors. Syntax: Actor1 --> Actor2 : Message\",\n      \"insertText\": \"-->\"\n    }\n  ]\n}"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 381,
    "completion_tokens": 307,
    "total_tokens": 688
  }
}
```

From the `content` attribute in the response, we can simply parse and forward the response to the language client:

```json
{
  "id": 1,
  "result": [
    {
      "label": "actor",
      "kind": 1,
      "detail": "Defines a new actor",
      "documentation": "Use 'actor' to define a new actor in your diagram. Syntax: actor ActorName",
      "insertText": "actor "
    },
    {
      "label": "Alice",
      "kind": 1,
      "detail": "Existing actor",
      "documentation": "Alice is an existing actor in your diagram. You can use this actor for new interactions.",
      "insertText": "Alice"
    },
    {
      "label": "Bob",
      "kind": 1,
      "detail": "Existing actor",
      "documentation": "Bob is an existing actor in your diagram. You can use this actor for new interactions.",
      "insertText": "Bob"
    },
    {
      "label": "->",
      "kind": 1,
      "detail": "Interaction",
      "documentation": "Use '->' to define a new interaction between two actors. Syntax: Actor1 -> Actor2 : Message",
      "insertText": "->"
    },
    {
      "label": "-->",
      "kind": 1,
      "detail": "Interaction",
      "documentation": "Use '-->' to define a new interaction between two actors. Syntax: Actor1 --> Actor2 : Message",
      "insertText": "-->"
    }
  ]
}
```

#### update: using GPT-4-turbo-preview

While exploring GPT-4 usage as an API endpoint, OpenAI released a preview version
of an enhanced GPT-4 version: GPT-4-turbo. Switching to this version unfortunately
slightly changes the response behaviour: Responses are now markdown-formatted,
meaning the JSON payload is wrapped in a Markdown code block, and GPT-4-turbo even
adds explanation text.

While adapting the initial prompt solved the issue of added explanations in most cases, it was
not possible to have GPT-4-turbo not wrap its responses in a Markdown code block.
As first workaround, the language server will now use the JSON response within
the Markdown markup elements for code blocks and possible explanations.


### language server

The implemented language server acts as a middleware, mostly forwarding requests and responses. It contains
business logic for the handling of file contents, since the majority of LSP requests from the editor contain URIs
to the currently viewed file, and the GPT-4 API can not access them.

As a first workaround, the language server caches the file contents of the currently viewed file, and sends
them along with each request. Since we need to include the message history in each request, it is more
straightforward to send the complete file contents with each request, and completely omit partial or delta
updates of the file.

Additionally, the language server defines its capabilities, and the client can choose which of these
capabilities it wants to use. In the beginning, only `completions` were tested (code completions or text suggestions).
After that part was working successfully, the language server has been enhanced to report further capabilities from the language
server protocol. The (smoke-)tested capabilities and their results are documented in the following paragraphs.

#### capabilities

##### document highlight

- highlights selected (key-) words in the complete document (or range)
- does not yet work in the editor (TODO: seems to work in addition to basic syntax highlighting only?)

##### diagnostics

- shows errors and warnings in the document
- needs extra steps, since the server pushes diagnostics to the client without the client requesting it
- possible solution: add explicit request for diagnostics on each file change

##### hover

- shows information about the token under the cursor (e.g. type of variable, signature of method)
- works mostly as intended, detailed information about token is returned
- using GPT-4-turbo, the complete line is explained often

##### references

- shows references of the selected token, e.g. usages of methods
- results are currently faulty: wrong line numbers are reported, not all references are found

##### code action

- show possible changes to the code, either to fix an error/warning or refactor code
- GPT-4 offered actions, while GPT-4-turbo did not suggest a single action yet, without further prompting or examples

##### document formatting

- allows the language server to format a document or section of it according to language formatting rules
- does not work yet: the suggested changes lead to duplication of lines

##### renaming

- allows renaming of symbols throughout documents of complete workspace
- does not work reliably: other symbol got renamed, and new name was inserted on blank lines


TODO: check for problems in position encoding (code action seems to be 1 line off, also references are sometimes in empty lines)

#### workspace folders

In addition to single documents, the language server specification allows the handling
of multiple documents within a folder (or 'workspace'). This allows the server to analyse
all files within a project, and perform its actions on all of them (e.g. renaming or finding
references across several files). The handling of these workspace folders and files is
left to the language server completely. Since the client will send notifications about opened,
modified and closed files only, the language server needs means to access the files
on the client's system. Our language server implementation will take care of this, and
forward the contents of the workspace files to the GPT-4 API. For bigger workspaces and
numerous files, this can lead to high input token consumption - this will be dealt with
later in case it becomes an issue.

##### completions using workspace folders for few-shot learning

In order to prompt the GPT-4 API for more meaningful responses (e.g. similar entity naming, structure of document,
typical notes), the contents of the workspace documents can be used in a few-shot learning approach. This means
the language server will scan the workspace folders for fitting files (PlantUML files), and add their contents
as system messages to each prompt. In case this approach works, the GPT-4 API will generate completions that
resemble parts of the workspace files.

The files for this proof of concept can be found in the folder `./assets/workspace-test/`. There are 3
sequence diagrams featuring short discussions about recommendations for operating systems. The language server
has then been triggered for completions of actors and their messages, and indeed the GPT-4 API produced
fitting suggestions like the following (for messages between the actors):

```json
{
  "id": 11,
  "result": {
    "isIncomplete": false,
    "items": [
      {
        "label": "Linux",
        "kind": 15,
        "detail": "Operating System",
        "documentation": "Linux is a family of open-source Unix-like operating systems based on the Linux kernel.",
        "insertText": "Linux"
      },
      {
        "label": "Mac",
        "kind": 15,
        "detail": "Operating System",
        "documentation": "MacOS is a series of proprietary graphical operating systems developed and marketed by Apple Inc.",
        "insertText": "Mac"
      }
    ]
  }
}
```

```json
{
  "id": 23,
  "result": {
    "isIncomplete": false,
    "items": [
      {
        "label": "Mac or Linux",
        "kind": 15,
        "detail": "Operating System Recommendation",
        "documentation": "Either a Mac or something that runs Linux well.",
        "insertText": "Either a Mac or something that runs Linux well."
      }
    ]
  },
  "jsonrpc": "2.0"
}
```

This kind of completion could not be triggered on each try. For this to work better,
the initial prompt could be adjusted.

### usage in editors

The language server has been tested with Visual Studio Code and Neovim. For Visual Studio Code an own extension
has been created, and for Neovim instructions are available. For both editors currently the language server is
only working when it is available on `PATH`. This can easily be achieved with the rust toolchain (`rustup`):

```bash 
# this command will install the lsp-gpt executable
cargo install --force --path .
```

Additionally, the OpenAI API key and the OrganisationId need to be available as environment variables for the
editor and ultimately the language server. Since it would be unsafe to permanently add the API key to the
environment variables, instructions how to have them available for the language server can be found in the following paragraphs.

#### Visual Studio Code

In order to use a language server in Visual Studio Code, an extension containing the language client has to be created.
This extension can be found in the subfolder `./clients/vscode`. It contains a very minimal implementation that only
defines the executable of the language server and the file types it is applicable to (`plantuml` in our case).

This file type `plantuml` needs to be defined in our extension first, since Visual Studio Code does not know it
natively. To define the file type, we add a `contributes` property to the file `package.json`, which acts as
a manifest for the extension. This `contributes` property describes the name of the language, the file
extensions and it requires a minimal language configuration (e.g. how line comments look like, which
brackets are used).

In order to run the extension for testing and debugging purposes, the following modules habe to be installed:

```bash
# install node and the node package manager (if not present yet)
sudo apt-get install npm nodejs
# install node package for languageclient
npm install vscode-languageclient
```

Additionally, a launch configuration has been created at `./.vscode/launch.json`.
This allows Visual Studio Code to create a new session or window without any of the user's plugins, and only the
lsp-gpt extension installed. To start this session, select "Run & Debug" (or press `Ctrl+Shift+D`), then select
"Run lsp-gpt extension" (the little green play button).

In the prototype language server, the API key is read out of a specific file (`secrets.txt`), but it is also
possible to provide the secrets as environment variables:

```bash
# assuming the Visual Studio Code binary is available on PATH as 'code'
OPENAI_API_KEY=abc OPENAI_ORG_ID=xyz code
```

#### Neovim

The integration into Neovim for prototyping purposes is straightforward: Neovim offers native support for
language servers, and lsp-gpt can be registered with the following command:

```vim
:lua vim.lsp.start({ name = 'lsp-gpt', cmd = { 'lsp-gpt' }, root_dir = vim.loop.cwd(), })
```

Similar to the extension in Visual Studio Code, we define a name for our language server, and the way
the editor can invoke it: as an executable.

## first conclusions

- GPT-4 is powerful and easy to use: already rather simple prompting turned it into a functional language server for completions
- further prompts and few-shot learning can influence/improve the responses even further
- invocations of GPT-4 API take quite a while: roughly between 8 and 14 seconds
- GPT-4-turbo is faster for most queries (3 to 10 seconds)
  - but needs further/different prompting, since most language server capabilities do not easily work as intended 
- enabling multiple capabilities leads to numerous requests that can pile up quickly
- best capability out of the box is completion (text suggestion), probably due to the document-completing nature of GPTs
- few-shot learning with example documents is possible, needs refinement and better prompting
- prompting should be different for each capability

