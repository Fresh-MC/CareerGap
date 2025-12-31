#!/usr/bin/env python3
"""
Resume Parser CLI - Extracts structured data from resume files.
Usage: python parser.py <resume_file_path>
Output: JSON to stdout only

Note: This parser uses spaCy directly instead of pyresparser due to
spaCy 3.x compatibility issues with pyresparser.
"""

import sys
import json
import os
import re

def output_error(message: str) -> None:
    """Output error as JSON and exit."""
    print(json.dumps({"error": message}))
    sys.exit(1)

def output_result(data: dict) -> None:
    """Output result as JSON and exit."""
    print(json.dumps(data))
    sys.exit(0)

def extract_text_from_pdf(pdf_path: str) -> str:
    """Extract text from PDF file."""
    from pdfminer.high_level import extract_text
    return extract_text(pdf_path)

def extract_text_from_docx(docx_path: str) -> str:
    """Extract text from DOCX file."""
    from docx import Document
    doc = Document(docx_path)
    return "\n".join([p.text for p in doc.paragraphs])

def extract_skills(text: str, nlp) -> list:
    """Extract skills from text using spaCy NER and pattern matching."""
    skill_patterns = [
        r'\b(Python|Java|JavaScript|TypeScript|C\+\+|C#|Ruby|Go|Rust|Swift|Kotlin|PHP|Scala|R|MATLAB)\b',
        r'\b(React|Angular|Vue|Node\.js|Django|Flask|Spring|Express|Rails|Laravel|\.NET)\b',
        r'\b(AWS|Azure|GCP|Docker|Kubernetes|Jenkins|Git|Linux|SQL|NoSQL|MongoDB|PostgreSQL|MySQL)\b',
        r'\b(Machine Learning|Deep Learning|NLP|Computer Vision|TensorFlow|PyTorch|Scikit-learn)\b',
        r'\b(HTML|CSS|REST|GraphQL|API|Microservices|Agile|Scrum|CI/CD|DevOps)\b',
        r'\b(Excel|PowerPoint|Word|Tableau|Power BI|Jira|Confluence|Slack)\b',
    ]
    
    skills = set()
    for pattern in skill_patterns:
        matches = re.findall(pattern, text, re.IGNORECASE)
        skills.update([m.strip() for m in matches])
    
    doc = nlp(text[:100000])
    for ent in doc.ents:
        if ent.label_ in ["ORG", "PRODUCT"]:
            if len(ent.text) < 30 and not any(c.isdigit() for c in ent.text[:3]):
                skills.add(ent.text.strip())
    
    return list(skills)[:50]

def extract_experience(text: str, nlp) -> list:
    """Extract work experience entries."""
    experience = []
    doc = nlp(text[:100000])
    
    orgs = []
    for ent in doc.ents:
        if ent.label_ == "ORG":
            org_name = ent.text.strip()
            if len(org_name) > 2 and len(org_name) < 50:
                orgs.append(org_name)
    
    seen = set()
    for org in orgs:
        if org.lower() not in seen:
            seen.add(org.lower())
            experience.append({"company": org})
    
    return experience[:10]

def extract_education(text: str) -> list:
    """Extract education entries."""
    education = []
    
    degree_patterns = [
        r'(Bachelor|Master|PhD|Ph\.D|B\.S\.|M\.S\.|B\.A\.|M\.A\.|MBA|B\.Tech|M\.Tech|B\.E\.|M\.E\.)[^\n,]*',
        r'(Computer Science|Engineering|Business|Mathematics|Physics|Chemistry|Biology|Economics)[^\n,]*degree',
    ]
    
    degrees_found = set()
    for pattern in degree_patterns:
        matches = re.findall(pattern, text, re.IGNORECASE)
        for m in matches:
            if isinstance(m, tuple):
                m = m[0]
            degrees_found.add(m.strip())
    
    for degree in list(degrees_found)[:5]:
        education.append({"degree": degree})
    
    return education

def extract_total_experience(text: str) -> float:
    """Try to extract years of experience."""
    patterns = [
        r'(\d+)\+?\s*years?\s*(?:of\s*)?experience',
        r'experience\s*(?:of\s*)?(\d+)\+?\s*years?',
        r'(\d+)\+?\s*years?\s*(?:in|of)\s*(?:software|development|engineering)',
    ]
    
    for pattern in patterns:
        match = re.search(pattern, text, re.IGNORECASE)
        if match:
            try:
                return float(match.group(1))
            except (ValueError, IndexError):
                pass
    return None

def parse_resume(file_path: str) -> dict:
    """Parse resume and return structured data."""
    try:
        import warnings
        warnings.filterwarnings("ignore")
        
        ext = os.path.splitext(file_path)[1].lower()
        if ext == ".pdf":
            raw_text = extract_text_from_pdf(file_path)
        elif ext in [".docx", ".doc"]:
            raw_text = extract_text_from_docx(file_path)
        elif ext == ".txt":
            # Handle plain text files (from raw_text input)
            with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                raw_text = f.read()
        else:
            return {
                "skills": [],
                "experience": [],
                "education": [],
                "total_experience": None,
                "raw_text": ""
            }
        
        if not raw_text or not raw_text.strip():
            return {
                "skills": [],
                "experience": [],
                "education": [],
                "total_experience": None,
                "raw_text": ""
            }
        
        import spacy
        nlp = spacy.load("en_core_web_sm")
        
        skills = extract_skills(raw_text, nlp)
        experience = extract_experience(raw_text, nlp)
        education = extract_education(raw_text)
        total_exp = extract_total_experience(raw_text)
        
        return {
            "skills": skills,
            "experience": experience,
            "education": education,
            "total_experience": total_exp,
            "raw_text": raw_text.strip()
        }
        
    except ImportError as e:
        output_error(f"Missing dependency: {str(e)}. Run: pip install -r requirements.txt")
    except Exception as e:
        output_error(f"Parse error: {str(e)}")

def main():
    import sys
    import traceback
    
    try:
        # Debug logging to stderr
        print(f"[parser.py] Python version: {sys.version}", file=sys.stderr)
        print(f"[parser.py] Python executable: {sys.executable}", file=sys.stderr)
        print(f"[parser.py] Arguments: {sys.argv}", file=sys.stderr)
        
        if len(sys.argv) < 2:
            output_error("Usage: python parser.py <resume_file_path>")
        
        file_path = sys.argv[1]
        print(f"[parser.py] Processing file: {file_path}", file=sys.stderr)
        
        if not os.path.isfile(file_path):
            output_error(f"File not found: {file_path}")
        
        ext = os.path.splitext(file_path)[1].lower()
        if ext not in [".pdf", ".docx", ".doc", ".txt"]:
            output_error(f"Unsupported file type: {ext}. Use PDF, DOCX, or TXT.")
        
        print(f"[parser.py] File extension: {ext}", file=sys.stderr)
        result = parse_resume(file_path)
        print(f"[parser.py] Parsing completed successfully", file=sys.stderr)
        print(f"[parser.py] Skills found: {len(result.get('skills', []))}", file=sys.stderr)
        print(f"[parser.py] Experience entries: {len(result.get('experience', []))}", file=sys.stderr)
        output_result(result)
        
    except Exception as e:
        # Send error details to stderr for debugging
        print(f"[parser.py] EXCEPTION: {str(e)}", file=sys.stderr)
        print(f"[parser.py] Traceback:", file=sys.stderr)
        print(traceback.format_exc(), file=sys.stderr)
        # Send error JSON to stdout for Rust to parse
        output_error(f"Exception: {str(e)}")

if __name__ == "__main__":
    main()
