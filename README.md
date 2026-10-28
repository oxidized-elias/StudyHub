<p align="center">
  <img src="logo.png" width="120" alt="StudyHub logo" />
</p>
<h1 align="center">StudyHub</h1>
<p align="center">
  Self-hosted Notes, Notebooks and AI Studying<br>
  Inspired by Gemini's learning tools and NotebookLM, with your own notes and choice of AI model<br>
  Import your notes, connect big ideas, and turn them into guided lessons and practice
</p>

<br>

## Features

* Import PDFs, images and text, individually or through Bulk Unknown Upload
* Review AI notes alongside the original upload or transcription before saving
* Organize notes by subject and course in an Obsidian vault
* Subject-wide BigIdeas indexes that connect related concepts across courses
* Bullet Point, Text Chunk and Combined note styles, with per-note settings
* Notebooks with folder trees and separate sources for learning and quizzes
* Guided learning, quizzes, flashcards, study guides and step-by-step practice
* A tutor that uses selected notes, explains sources and generates practice problems
* Concept mastery tracking, targeted quizzes, closed-book recall and a problem journal
* Cloud model profiles and local or remote Ollama servers
* Saved history, correction lessons, notifications and live session logs
* One integrated server and Material UI interface, with an interactive tutorial

## Getting Started

### GitHub Container Registry (Recommended):

Dependencies:

* Docker
* An AI provider or a reachable model server, configured after startup

```bash
docker pull ghcr.io/oxidized-elias/studyhub:sha-944ad9ff241a50c0e92013c5af2ada8db0e901ea
docker run -d --name studyhub --init --restart unless-stopped \
  -p 127.0.0.1:8082:8082 \
  -v studyhub_data:/app/data \
  -v studyhub_vault:/vault \
  --add-host=host.docker.internal:host-gateway \
  ghcr.io/oxidized-elias/studyhub:sha-944ad9ff241a50c0e92013c5af2ada8db0e901ea
```

Open **http://localhost:8082**. The command above uses the published commit image; use `ghcr.io/oxidized-elias/studyhub:latest` for the latest successful `main` build. Both named volumes keep your data when the container is replaced.

If running the application with a domain, add these environment options to `docker run` and configure your HTTPS reverse proxy to forward to port `8082`:

```bash
-e STUDYHUB_URL=https://study.example.com \
-e STUDYHUB_USER=admin \
-e STUDYHUB_PASSWORD='your-password'
```

### Dockerfile:

Dependencies:

* Docker
* git

```bash
git clone https://github.com/oxidized-elias/StudyHub.git && cd StudyHub
docker build -t studyhub:local .
docker run -d --name studyhub --init --restart unless-stopped \
  -p 127.0.0.1:8082:8082 \
  -v studyhub_data:/app/data \
  -v studyhub_vault:/vault \
  --add-host=host.docker.internal:host-gateway \
  studyhub:local
```

Or use Docker Compose from the cloned repository:

```bash
docker compose up -d --build
```

See [Docker setup](docs/DOCKER.md) for host vault mounts, remote access, backups and model connections.

### Manual Install:

Dependencies:

* git
* Node.js 22 or newer
* Rust and Cargo
* A C compiler for bundled SQLite
* Poppler (`pdftoppm` and `pdftotext`) for PDF imports
* An AI provider or a reachable model server

```bash
git clone https://github.com/oxidized-elias/StudyHub.git && cd StudyHub
node server.mjs --release # Builds and starts the application
```

Open **http://127.0.0.1:8082**. The first build downloads Cargo dependencies. No npm install is needed to run StudyHub. Ctrl+C stops the server.

Other scripts:

| Command | Description |
| --- | --- |
| `npm start` | Build and start the debug application |
| `./start.sh` | Start through the Node launcher |
| `node server.mjs --release` | Build and start the optimized application |
| `npm run build` | Build the optimized application without starting it |
| `node server.mjs --release --no-build` | Start an existing optimized build |
| `npm test` | Run launcher and Rust tests |
| `npm run test:ui` | Run UI regression checks; see [development setup](docs/DEVELOPMENT.md) for test dependencies |
| `npm run test:docker-config` | Check Docker configuration and health checks |
| `npm run test:docker` | Test container startup and persistence using Docker |

## First Use

1. Open **AI Models & Settings**, save a model profile and test its connection. Configure **Note Import Models** separately for note writing and image/PDF reading; page reading requires a vision-capable model.
2. Choose your Obsidian vault location and default Note Style. With the Docker commands above, the vault location inside the application is **`/vault`**.
3. Upload notes, compare the AI draft with the source, make corrections and approve. Bulk imports offer an optional **Skip Human Intervention** setting.
4. Create a notebook and select individual notes or folders. Choose which sources to learn from and which to use for quizzes.
5. Start a lesson, generate practice or chat with your notes. Use the interactive tutorial for a walkthrough; replay it from Settings.

# FAQ

## Data Storage

Manual installs store settings, provider keys, original uploads, correction lessons and SQLite history under `data/`. The default Obsidian vault is `data/vault/`; you can choose another folder in Settings. User data and keys are git-ignored.

Docker stores application data in `/app/data` and approved notes in `/vault`. The commands above attach persistent volumes to both. Back up both locations. For an existing Obsidian folder, see [host vault setup](docs/DOCKER.md#use-a-host-obsidian-vault).

## Application Security?

StudyHub is designed as a single-user application. Set `STUDYHUB_PASSWORD` to enable HTTP Basic authentication, and use HTTPS through your reverse proxy when exposing it. You can also manage access through a service such as Cloudflare Zero Trust. Provider keys are stored in the server's data directory.

See [configuration and operations](docs/CONFIGURATION.md) for authentication, environment settings and backups.

## Can I Use a Local Model?

Yes. Configure an Ollama server on this computer or any reachable host. In Docker, `localhost` refers to the container; use a remote server address or `http://host.docker.internal:11434` for Ollama on the Docker host. The model server must accept connections from the container. StudyHub does not bundle or download model weights.

Cloud profiles support OpenAI, Claude, Gemini and OpenAI-compatible endpoints. Notes imports have their own provider and model selections.

## Can It Run Completely Offline?

Local models handle inference on your server, but browser libraries currently load from pinned CDNs. The browser still needs internet access for those assets.

## Where Are the Transcriptions?

Open a note and expand **View Source · Transcription or Original Upload**. Switch between **AI Transcription** and **Actual Upload**. Transcriptions are kept separately from the generated Markdown note.

## Where Is the Documentation?

* [User guide](docs/USER_GUIDE.md): notes, notebooks, review and study tools
* [Application map](docs/Application%20Map.md): Obsidian-compatible overview for contributors and future models
* [Architecture](docs/STUDY_SUITE.md): application modules, storage and integrations
* [Docker setup](docs/DOCKER.md): deployment, vault mounts and image publishing
* [Configuration](docs/CONFIGURATION.md): environment settings and operations
* [API reference](docs/API.md): application endpoints
* [Development](docs/DEVELOPMENT.md): source layout, test dependencies and checks
* [Learning workflows](docs/LEARNING_WORKFLOWS.md): guided practice, corrections and mastery tracking

<br>

## Credits

* [Google](https://google.com) for creating NotebookLM
* [oxidized-elias](https://github.com/oxidized-elias) for creating StudyHub _idea_
* [Magnific - flaticon](https://www.flaticon.com/free-icons/education "Education icons") for the icon
* [Claude Sonnet 5](https://claude.ai) "Programmer"
* [GPT-6.1-Sol](https://chatgpt.com) "Lead" Programmer
