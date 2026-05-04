#!/usr/bin/env python3
"""
Company Research Loader for Pelendur.

Integrates with NotebookLM deep research to generate structured company
overview.md files in knowledge/companies/<company>/.

Usage:
  python3 company-research.py "Company Name" [--notebook-id <uuid>]

If --notebook-id is omitted, the script lists your notebooks so you can pick one.
The script:
  1. Runs NotebookLM deep research on the company
  2. Generates a structured overview.md
  3. (Optional) Loads it into the Pelendur knowledge graph
"""

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

# ─── Paths ───────────────────────────────────────────────────────────────
PROJECT_ROOT = Path("/mnt/c/Proyectos/Pelendur")
KNOWLEDGE_DIR = PROJECT_ROOT / "knowledge"
COMPANIES_DIR = KNOWLEDGE_DIR / "companies"
NOTEBOOKLM_BIN = os.path.expanduser("~/.local/bin/notebooklm-mcp")

# ─── Helpers ─────────────────────────────────────────────────────────────

def run_notebooklm(*args: str, timeout: int = 300) -> str:
    """Run notebooklm-mcp CLI and return stdout."""
    cmd = [NOTEBOOKLM_BIN] + list(args)
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        if result.returncode != 0:
            raise RuntimeError(f"notebooklm-mcp error: {result.stderr.strip()}")
        return result.stdout.strip()
    except subprocess.TimeoutExpired:
        raise RuntimeError(f"notebooklm-mcp timed out after {timeout}s")

def list_notebooks() -> list[dict]:
    """List all available notebooks."""
    raw = run_notebooklm("list", "--json")
    if not raw:
        return []
    try:
        data = json.loads(raw)
        return data if isinstance(data, list) else [data]
    except json.JSONDecodeError:
        print("⚠️  Could not parse notebook list JSON")
        print(f"Raw: {raw[:500]}")
        return []

def company_dir_name(name: str) -> str:
    """Normalize company name to directory name."""
    return name.lower().strip().replace(" ", "-").replace(".", "")

def extract_company_info(text: str, company_name: str) -> dict:
    """
    Extract structured company info from NotebookLM research output
    using simple heuristics. Falls back to generating a skeleton.
    """
    info = {
        "company_name": company_name,
        "industry": None,
        "description": None,
        "culture": None,
        "tech_stack": None,
        "strategic_angle": None,
        "key_challenges": [],
        "products": [],
        "competitors": [],
        "interview_tips": [],
    }

    lines = text.split("\n")
    current_section = ""

    for line in lines:
        lower = line.strip().lower()

        # Section headers
        if re.match(r"^#{1,4}\s+(industry|about|overview)\b", lower):
            current_section = "industry"
            continue
        elif re.match(r"^#{1,4}\s+(culture|values|work\s*culture)\b", lower):
            current_section = "culture"
            continue
        elif re.match(r"^#{1,4}\s+(tech\s*stack|technology|stack)\b", lower):
            current_section = "tech_stack"
            continue
        elif re.match(r"^#{1,4}\s+(strategy|strategic|competitive\s*angle)\b", lower):
            current_section = "strategic"
            continue
        elif re.match(r"^#{1,4}\s+(challenges|problems|key\s*challenges)\b", lower):
            current_section = "challenges"
            continue
        elif re.match(r"^#{1,4}\s+(products|services|offerings)\b", lower):
            current_section = "products"
            continue
        elif re.match(r"^#{1,4}\s+(competitors|competition|landscape)\b", lower):
            current_section = "competitors"
            continue
        elif re.match(r"^#{1,4}\s+(tips|interview\s*tips|advice)\b", lower):
            current_section = "tips"
            continue

        # Key-value lines: "Industry: ..." or "- Industry: ..."
        kv_match = re.match(r"^-?\s*\*{0,2}(industry|description|culture|tech\s*stack|strategic\s*angle|stack)\*{0,2}\s*:\s*(.+)", line.strip(), re.IGNORECASE)
        if kv_match:
            key = kv_match.group(1).strip().lower()
            value = kv_match.group(2).strip()
            if "industry" in key:
                info["industry"] = value
            elif "description" in key:
                info["description"] = value
            elif "culture" in key:
                info["culture"] = value
            elif "tech stack" in key or "stack" in key:
                info["tech_stack"] = value
            elif "strategic angle" in key or "strategy" in key:
                info["strategic_angle"] = value
            continue

        # Bullet items under sections
        stripped = line.strip()
        if stripped.startswith("- ") or stripped.startswith("* ") or re.match(r"^\d+\.\s+", stripped):
            content = re.sub(r"^[-*\d.]+\s+", "", stripped).strip()
            if not content:
                continue
            if current_section in ("challenges",):
                info["key_challenges"].append(content)
            elif current_section in ("products", "services"):
                info["products"].append(content)
            elif current_section in ("competitors", "competition"):
                info["competitors"].append(content)
            elif current_section in ("tips", "interview"):
                info["interview_tips"].append(content)

        # Non-bullet content under strategy section
        if current_section == "strategic" and stripped and not stripped.startswith("#") and not stripped.startswith("-"):
            # Could be a multi-line description
            if info["strategic_angle"]:
                info["strategic_angle"] += " " + stripped
            elif len(stripped) > 20:  # meaningful text
                info["strategic_angle"] = stripped

    return info

def generate_overview_md(info: dict) -> str:
    """Generate structured overview.md from extracted info."""
    md = f"# {info['company_name']} Research\n\n"

    if info.get("industry"):
        md += f"- **Industry**: {info['industry']}\n"
    if info.get("description"):
        md += f"- **Description**: {info['description']}\n"
    if info.get("culture"):
        md += f"- **Culture**: {info['culture']}\n"
    if info.get("tech_stack"):
        md += f"- **Tech Stack**: {info['tech_stack']}\n"
    if info.get("strategic_angle"):
        md += f"- **Strategic Angle**: {info['strategic_angle']}\n"

    if info.get("key_challenges"):
        md += "\n## Key Challenges\n"
        for c in info["key_challenges"]:
            md += f"- {c}\n"

    if info.get("products"):
        md += "\n## Products\n"
        for p in info["products"]:
            md += f"- {p}\n"

    if info.get("competitors"):
        md += "\n## Competitors\n"
        for c in info["competitors"]:
            md += f"- {c}\n"

    if info.get("interview_tips"):
        md += "\n## Interview Tips\n"
        for t in info["interview_tips"]:
            md += f"- {t}\n"

    return md

def save_overview(info: dict) -> Path:
    """Save the overview.md file."""
    dir_name = company_dir_name(info["company_name"])
    dir_path = COMPANIES_DIR / dir_name
    dir_path.mkdir(parents=True, exist_ok=True)

    md = generate_overview_md(info)
    md_path = dir_path / "overview.md"
    md_path.write_text(md, encoding="utf-8")
    return md_path

# ─── Main ────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Pelendur Company Research Loader")
    parser.add_argument("company", help="Company name to research")
    parser.add_argument("--notebook-id", help="NotebookLM notebook ID (omit to list and pick)")
    parser.add_argument("--skip-research", action="store_true",
                        help="Skip NotebookLM research, just create skeleton")
    args = parser.parse_args()

    company = args.company.strip()

    # Create skeleton
    info = {
        "company_name": company,
        "industry": None,
        "description": None,
        "culture": None,
        "tech_stack": None,
        "strategic_angle": None,
        "key_challenges": [],
        "products": [],
        "competitors": [],
        "interview_tips": [
            f"Research common interview questions for {company}",
            f"Align STAR stories with {company}'s industry and challenges",
            f"Emphasize relevant tech stack experience",
        ],
    }

    if not args.skip_research:
        # Find or use notebook
        notebook_id = args.notebook_id
        if not notebook_id:
            print("📚 Listing available notebooks...")
            notebooks = list_notebooks()
            if not notebooks:
                print("⚠️  No NotebookLM notebooks found. Creating skeleton only.")
                args.skip_research = True
            else:
                print("\nAvailable notebooks:")
                for i, nb in enumerate(notebooks[:10]):
                    if isinstance(nb, dict):
                        name = nb.get("name", nb.get("title", "?"))
                        uid = nb.get("id", nb.get("uuid", "?"))
                    else:
                        name = str(nb)
                        uid = "?"
                    print(f"  [{i+1}] {name}  ({uid})")
                print()
                try:
                    choice = int(input("Select notebook number (or 0 to skip): ").strip())
                    if 1 <= choice <= len(notebooks):
                        selected = notebooks[choice - 1]
                        notebook_id = selected.get("id", selected.get("uuid", ""))
                        print(f"   Selected: {notebook_id}")
                    else:
                        print("   Skipping research.")
                        args.skip_research = True
                except (ValueError, EOFError, IndexError):
                    print("   Skipping research.")
                    args.skip_research = True

        if not args.skip_research and notebook_id:
            print(f"🔬 Running NotebookLM deep research on '{company}'...")
            print(f"   Notebook: {notebook_id}")
            print(f"   This may take several minutes...")
            try:
                research_text = run_notebooklm(
                    "research",
                    "--notebook-id", notebook_id,
                    "--prompt", f"Research the company {company} for interview preparation. Include: industry, description, culture, tech stack, strategic angle, key challenges, products, competitors. Provide structured information.",
                    timeout=900,  # 15 min for deep research
                )
                print(f"   ✅ Research complete ({len(research_text)} chars)")

                # Extract structured info
                extracted = extract_company_info(research_text, company)
                # Keep what was extracted, fill from defaults for what wasn't
                for key in info:
                    if extracted.get(key):
                        info[key] = extracted[key]
                print(f"   Extracted: industry={bool(info['industry'])}, "
                      f"culture={bool(info['culture'])}, "
                      f"tech_stack={bool(info['tech_stack'])}, "
                      f"challenges={len(info['key_challenges'])}")

            except RuntimeError as e:
                print(f"⚠️  Research failed: {e}")
                print("   Creating skeleton file instead.")

    # Save
    md_path = save_overview(info)
    print(f"\n📄 Saved: {md_path}")

    # Show preview
    print("\n── Preview ──")
    print(md_path.read_text(encoding="utf-8"))
    print("─────────────\n")

    print("💡 To load into Pelendur knowledge graph, run:")
    dir_name = company_dir_name(company)
    print(f"   cd {PROJECT_ROOT} && python3 -c \"")
    print(f"from ghostai_pilot.knowledge.company import CompanyLoader, CompanyResearch")
    print(f"loader = CompanyLoader('knowledge')")
    print(f"research = CompanyResearch.from_markdown('knowledge/companies/{dir_name}/overview.md')")
    print(f"print('Parsed OK:', research.company_name)\"")
    print()
    print(f"Or from the Pelendur UI, use the 'Refresh Company Research' command.")


if __name__ == "__main__":
    main()
