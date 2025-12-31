@echo off
REM Setup script for resume_parser Python environment (Windows)

echo Creating Python virtual environment...
python -m venv venv

echo Activating virtual environment...
call venv\Scripts\activate.bat

echo Installing dependencies...
pip install -r requirements.txt

echo Downloading spaCy language model...
python -m spacy download en_core_web_sm

echo.
echo Setup complete! To use the parser:
echo   1. Activate: venv\Scripts\activate
echo   2. Run: python parser.py path\to\resume.pdf
echo.
pause
