# Resume Parser

Standalone Python utility for extracting structured data from resume files (PDF/DOCX).

## Setup

```bash
cd resume_parser
setup.bat
```

Or manually:

```bash
python -m venv venv
venv\Scripts\activate
pip install -r requirements.txt
python -m spacy download en_core_web_sm
```

## Usage

```bash
python parser.py path\to\resume.pdf
```

Output is **JSON only** to stdout:

```json
{
  "skills": ["Python", "JavaScript", "SQL"],
  "experience": [{"company": "Tech Corp", "role": "Engineer"}],
  "education": [{"degree": "BS Computer Science", "institution": "MIT"}],
  "total_experience": 5.0,
  "raw_text": "Full extracted text..."
}
```

## Integration with Rust

The Rust backend calls this script via `std::process::Command`. See `backend/src/agent/resume_parser.rs` for the integration code.

## Notes

- Python is used **only** for parsing - no business logic
- All intelligence and decision-making remains in Rust
- Script exits after execution (not a service)
