Core Functionalities:-
1) Prompt-based Code Generation

--Users can type a prompt which is sent to the backend.

--The generated code is automatically written into main.rs.

2) Auto File Creation from Prompt

--The generated code from the LLM is saved to Rust_Project/src/main.rs.

--A basic Cargo.toml is also generated.

3) Auto Dependency Inference

--While saving main.rs, the code is scanned for common crates (e.g., tokio, serde, rand).

--These are added dynamically to Cargo.toml.

--If Cargo.toml is already open, it is refreshed in real-time.

4) Project Tree View (File Explorer)

--Recursive view of all files and folders inside Rust_Project/.
--Icons shown based on file type to create or rename the files.

-----> File & Folder Operations
5) Right-Click Context Menu on Files and Folders

 -New File

 -New Folder

 -Rename

 -Delete

6) Tab-based Multi-file Editor

--Clicking a file opens it in a new tab.

--Multiple files can be opened simultaneously.

--Switching between tabs works smoothly.

--Tabs can be closed via the ✕ button.

---> Editor Features
7) Code Editor with Line Numbers

--Line numbers displayed in a gutter.

--Scrolls in sync with the code.

--Line count adjusts dynamically.


8) Save File Button

--Appears below the editor.

--Updates file content.

--Triggers dependency inference logic if main.rs is edited.

10) LLM Integration Features
--Send Prompt

--A top bar input lets user type any natural-language prompt.

--Clicking "Run" button sends the prompt to a backend LLM.

11) Terminal Output Panel

--A bottom panel ( Terminal Output) displays output from:

--Code generation

--Compilation (cargo run)

--Runtime errors

12) Auto Scroll Option

--Controlled via settings on the right panel.


---> Settings Panel
13)Dark/Light Theme Toggle

--Dynamically updates the visual style.

--Stored within session (default is dark).

14) Auto-Scroll Toggle

--Controls whether the output panel auto-scrolls.

15) Live Phase Status

--Shows whether prompt is processing, completed, or failed in right panel beside Status.

16) Build & Run Button

--Executes cargo run inside Rust_Project.

--Output displayed in terminal panel.
