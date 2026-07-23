from __future__ import annotations

import unittest

from chefbar.palette import Action, fuzzy_score, rank_actions


def action(title: str, keywords: str = "") -> Action:
    return Action(title, "", "STIL", keywords, lambda _query: None)


class PaletteTests(unittest.TestCase):
    def test_exact_match_ranks_before_fuzzy_match(self) -> None:
        actions = [
            action("Start desktop"),
            action("Deel lokale bestanden", "desktop"),
        ]
        self.assertEqual(rank_actions(actions, "desktop")[0].title, "Start desktop")

    def test_ordered_character_fuzzy_match(self) -> None:
        self.assertIsNotNone(fuzzy_score("str dsk", action("Start desktop")))

    def test_missing_characters_do_not_match(self) -> None:
        self.assertIsNone(fuzzy_score("xyz", action("Start Commander")))


if __name__ == "__main__":
    unittest.main()
