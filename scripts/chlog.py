import argparse
import re
import subprocess
import sys
from collections import defaultdict

TYPE_ORDER = ['feat', 'fix', 'docs', 'style', 'refactor', 'perf', 'test', 'chore', 'other']

types = {
    'feat': 'Features',
    'fix': 'Fixes',
    'docs': 'Documentation',
    'style': 'Styles',
    'refactor': 'Refactoring',
    'perf': 'Performance',
    'test': 'Testing',
    'chore': 'Chore',
    'other': 'Other',
}

def run_git(args: list[str]) -> str:
    try:
        return subprocess.check_output(['git'] + args, stderr=subprocess.DEVNULL, text=True).strip()
    except subprocess.CalledProcessError as e:
        print(f"Git error: {e}", file=sys.stderr)
        sys.exit(1)


def get_last_tag() -> str | None:
    try:
        tag = run_git(['describe', '--tags', '--abbrev=0'])
        return tag
    except SystemExit:
        return None


def get_commits_since(tag: str | None) -> list[tuple[str, str]]:
    if tag:
        rev_range = f'{tag}..HEAD'
    else:
        rev_range = 'HEAD'

    output = run_git(['log', rev_range, '--format=%H%n%B%n---COMMIT_END---'])
    if not output:
        return []

    raw_commits = output.split('---COMMIT_END---')
    commits = []
    for chunk in raw_commits:
        chunk = chunk.strip()
        if not chunk:
            continue
        lines = chunk.splitlines()
        if not lines:
            continue
        hash_full = lines[0].strip()

        message = '\n'.join(lines[1:]).strip()
        commits.append((hash_full, message))  # pyright: ignore[reportUnknownMemberType]
    return commits  # pyright: ignore[reportUnknownVariableType]


def parse_commit_message(message: str) -> dict | None:  # pyright: ignore[reportUnknownParameterType, reportMissingTypeArgument]
    lines = message.splitlines()
    if not lines:
        return None
    subject = lines[0].strip()
    body = lines[1:] if len(lines) > 1 else []

    pattern = re.compile(r'^(?P<type>[a-z]+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?: (?P<description>.+)')
    match = pattern.match(subject)

    if not match:
        return {  # pyright: ignore[reportUnknownVariableType]
            'type': 'other',
            'scope': None,
            'breaking': False,
            'description': subject,
            'body': body
        }

    type_ = match.group('type')
    scope = match.group('scope')
    breaking = bool(match.group('breaking'))
    description = match.group('description')

    if not breaking:
        for line in body:
            if line.strip().startswith('BREAKING CHANGE:'):
                breaking = True
                break

    return {  # pyright: ignore[reportUnknownVariableType]
        'type': type_,
        'scope': scope,
        'breaking': breaking,
        'description': description,
        'body': body
    }


def generate_release_notes(commits: list[tuple[str, str]]) -> str:
    groups = defaultdict(list)  # pyright: ignore[reportUnknownVariableType]
    breaking_changes = []

    for hash_full, message in commits:
        parsed = parse_commit_message(message)  # pyright: ignore[reportUnknownVariableType]
        if parsed is None:
            continue

        short_hash = hash_full[:7]
        entry = {  # pyright: ignore[reportUnknownVariableType]
            'hash': short_hash,
            'description': parsed['description'],
            'scope': parsed['scope'],
            'breaking': parsed['breaking'],
            'type': parsed['type']
        }
        if parsed['breaking']:
            breaking_changes.append(entry)  # pyright: ignore[reportUnknownMemberType]
        else:
            groups[parsed['type']].append(entry)  # pyright: ignore[reportUnknownMemberType]

    output_lines = []

    if breaking_changes:
        output_lines.append("## BREAKING CHANGES")  # pyright: ignore[reportUnknownMemberType]
        for item in breaking_changes:  # pyright: ignore[reportUnknownVariableType]
            scope_str = f"({item['scope']})" if item['scope'] else ""
            output_lines.append(f"- {item['description']} {scope_str}")  # pyright: ignore[reportUnknownMemberType]
        output_lines.append("")  # pyright: ignore[reportUnknownMemberType]

    for type_ in TYPE_ORDER:
        if type_ not in groups:
            continue
        items = groups[type_]  # pyright: ignore[reportUnknownVariableType]
        header = types[type_]
        output_lines.append(f"### {header}")  # pyright: ignore[reportUnknownMemberType]
        for item in items:  # pyright: ignore[reportUnknownVariableType]
            scope_str = f"({item['scope']})" if item['scope'] else ""
            output_lines.append(f"- {item['description']} {scope_str}")  # pyright: ignore[reportUnknownMemberType]
        output_lines.append("")  # pyright: ignore[reportUnknownMemberType]

    return "\n".join(output_lines)  # pyright: ignore[reportUnknownArgumentType]


def main():
    parser = argparse.ArgumentParser(description='Generate release notes from git commits.')
    parser.add_argument('--output', '-o', help='Write output to file instead of stdout')  # pyright: ignore[reportUnusedCallResult]
    parser.add_argument('--since', help='Git tag/ref to start from (overrides auto-detection)')  # pyright: ignore[reportUnusedCallResult]
    args = parser.parse_args()

    if args.since:  # pyright: ignore[reportAny]
        tag = args.since  # pyright: ignore[reportAny]
    else:
        tag = get_last_tag()
        if tag:
            print(f"Using last tag: {tag}", file=sys.stderr)
        else:
            print("No tags found, using all commits.", file=sys.stderr)

    commits = get_commits_since(tag)
    if not commits:
        print("No commits found.", file=sys.stderr)
        sys.exit(0)

    notes = generate_release_notes(commits)

    if args.output:  # pyright: ignore[reportAny]
        with open(args.output, 'w', encoding='utf-8') as f:  # pyright: ignore[reportAny]
            _ = f.write(notes)
        print(f"Release notes written to {args.output}", file=sys.stderr)  # pyright: ignore[reportAny]
    else:
        print(notes)


if __name__ == '__main__':
    main()
