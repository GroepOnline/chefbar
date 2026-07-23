"""Pure command-palette registry and fuzzy ranking for ChefBar."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable


@dataclass(frozen=True)
class Action:
    title: str
    meta: str
    stamp: str
    keywords: str
    run: Callable[[str], None]
    needs_query: bool = False
    key: str = ""
    section: str = "Acties"
    shortcut: str = "↵"
    destructive: bool = False
    needs_text: bool = False
    pinned: bool = False

    def matches(self, query: str) -> bool:
        return fuzzy_score(query, self) is not None


def fuzzy_score(query: str, action: Action) -> int | None:
    """Rank ordered-character matches; exact words and prefixes win."""
    needle = " ".join(query.lower().split())
    if not needle:
        return 0
    haystack = " ".join(
        (action.title, action.meta, action.section, action.keywords)
    ).lower()
    if needle in haystack:
        return 1000 - haystack.index(needle)
    words = haystack.split()
    if all(any(word.startswith(token) for word in words) for token in needle.split()):
        return 700
    position = -1
    gaps = 0
    for char in needle:
        next_position = haystack.find(char, position + 1)
        if next_position < 0:
            return None
        if position >= 0:
            gaps += next_position - position - 1
        position = next_position
    return max(1, 500 - gaps)


def rank_actions(actions: list[Action], query: str, limit: int = 9) -> list[Action]:
    ranked: list[tuple[int, int, Action]] = []
    for index, action in enumerate(actions):
        score = fuzzy_score(query, action)
        if score is not None:
            ranked.append((score, -index, action))
    ranked.sort(key=lambda item: (item[0], item[1]), reverse=True)
    return [item[2] for item in ranked[:limit]]
