# lsp-gpt

... let's use GPT-4 as language server!

## what is it?

This project contains a language server (LS) implementation that uses the GPT-4 API in order
to provide suggestions according to the language server protocol (LSP) specification.
Additionally it contains clients or usage instructions for Visual Studio Code and Neovim.

## what is still missing?

To be done before the next iteration:

- explore GPT behaviour for additional LSP features: highlighting, diagnostics/errors, fixing
- enhance prompt with few-shot learning: add possibility to provide example diagrams
  - explore working on workspace level instead of document level only: other existing diagrams can be used as example
- decide on direction:
  - create a fully featured LSP (as easily as possible)
  - train it towards meaningful completions (from examples), e.g. fitting names/identifiers, desired syntax


## what has been done?

The following steps have been achieved on a prototype level. Details can be found in the linked sub sections.

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

From the `content` atribute in the response, we can simply parse and forward the response to the language client:

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


### language server

The implemented language server acts as a middleware, mostly forwarding requests and responses. It contains
business logic for the handling of file contents, since LSP requests from the editor mainly contain paths
to the currently viewed file, and the GPT-4 API can not access them.

As a first workaround, the language server caches the file contents of the currently viewed file, and sends
them along with each request. Since we need to include the message history in each request, it is more
straightforward to send the complete file contents with each request, and completely omit partial or delta
updates of the file.

Additionally the language server defines its capabilities, and the client can choose which of these
capabilities it wants to use. In the beginning, only `completions` were tested (code completions or text suggestions).
After that part was working successfully, the language server also reported further capabilities from the language
server protocol. The tested capabilities and their results are documented in the following paragraphs.

#### capability: document highlight provider

- highlights selected (key-) words in the complete document (or range)
- does not yet work in the editor (TODO: seems to work in addition to basic syntax highlighting only?)

#### capability: diagnostic provider

- shows errors and warnings in the document
- needs extra steps, since the server pushes diagnostics to the client without the client requesting it
- possible solution: add explicit request for diagnostics on each file change

#### capability: hover provider

- shows information about the token under the cursor (e.g. type of variable, signature of method)
- works as intended, detailed information about token is returned

#### capability: references provider

- shows references of the selected token, e.g. usages of methods
- request to GPT-4 API runs into timeout currently (TODO: investigate)


TODO: describe additional capabilities
TODO: check for problems in position encoding (code action seems to be 1 line off)

### usage in editors

The language server has been tested with Visual Studio Code and Neovim. For Visual Studio Code an own extension
has been created, and for Neovim instructions are available. For both editors currently the language server is
only working when it is available on `PATH`. This can easily be achieved with the rust toolchain (`rustup`):

```bash 
# this command will install the lsp-gpt executable
cargo install --force --path .
```

Additionally the OpenAI API key and the OrganisationId need to be available as environment variables for the
editor and ultimatively the language server. Since it would be unsafe to permanently add the API key to the
environment variables, instructions how to have them avaiable for the language server can be found in the following paragraphs.

#### Visual Studio Code

In order to use a language server in Visual Studio Code, an extension containing the language client has to be created.
This extension can be found in the subfolder `./clients/vscode`. It contains a very minimal implementation that only
defines the executable of the language server and the file types it is applicable to (`plantuml` in our case).

This file type `plantuml` needs to be defined in our extension first, since Visual Studio Code does not know it
natively. To define the file type, we add a `contributes` property to the file `package.json`, which acts as
a manifest for the extension. This `contributes` property describes the name of the language, the file
extensions and it requires a minimal language configuration (e.g. how line comments look like, which
brackets are used).

To run the extension for testing and debugging purposes, a launch configuration has been created at `./.vscode/launch.json`.
This allows Visual Studio Code to create a new session or window without any of the user's plugins, and only the
lsp-gpt extension installed. To start this session, select "Run & Debug" (or press `Ctrl+Shift+D`), then select
"Run lsp-gpt extension" (the little green play button).

If the initial Visual Studio Code instance has been started with the environment variables, it will pass them to
all its child processes, including the lsp-gpt language server:

```bash
# assuming the Visual Studio Code binary is available on PATH as 'code'
OPENAI_API_KEY=abc OPENAI_ORG_ID=xyz code
```

#### Neovim

The integration into Neovim for prototyping purposes is much easier: Neovim offers native support for
language servers, and lsp-gpt can be registered with the following command:

```vim
:lua vim.lsp.start({ name = 'lsp-gpt', cmd = { 'lsp-gpt' }, root_dir = vim.loop.cwd(), })
```

Similar to the extension in Visual Studio Code, we define a name for our language server, and how
the editor can invoke it: as an executable.

## first conclusions

- GPT-4 is powerful and easy to use: already rather simple prompting turned it into a functional language server
- further prompts and few-shot learning can influence/improve the responses even further
- invocations of GPT-4 API take quite a while: roughly between 8 and 14 seconds

<hr />
<hr />

# ⚠ old notes, to be removed ⚠

check if and what exactly for this setup is needed:

```bash
sudo apt-get install npm nodejs
npm install vscode-languageclient
```

