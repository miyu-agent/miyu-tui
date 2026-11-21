//! 界面上的字（[`super::Texts`]）照哪几个键读（施工 LW-9，`from_json`）：字段多，单放一份。

use super::Texts;

crate::from_json::record! {
    Texts {
        tips, markdown, untitled, side_jumps, slot_unknown, queued_withdraw, side_id, side_cwd,
        side_recent, side_context, side_context_value, side_usage, side_total, side_split, side_hit,
        side_compactions, side_breaks, side_cost, jobs, shell, prefix, help, widget, usage, status,
        tool_list, context_box, settings, drawer, paste_label, attach, mention, skill_word,
        clipboard_unreadable, clipboard_text_only, attach_deny_only, answer_files_lost,
        clipboard_empty, home_placeholder, archived_placeholder, levels, done, speed, copied,
        copy_failed, nothing_to_copy, connecting, no_core_bin, core_failed, reconnecting, turn_cut,
        missing_core, not_connected, start_core, switching, crash_saved, notify, refused,
        interrupted, failed, reason_with, status_hints, retry_after, retry, older, older_failed,
        exit_hint, input_cleared, too_long, too_long_attached, editing, open, esc_hint,
        esc_agents_hint, no_clear_running, stashed, done_usage, refusal_hints, recap, models,
        rename, compact_line, goal, files, loops, feature_tree, history, usage_cell, jump, menu,
        compaction, dream, unread, error_classes, figure_pending, sent, shown, mascot_failed,
        look_failed, console_cjk, icons_changed, undone, undo_restore, undo_compactions,
        undo_clears, restored, undo_files, stopped_jobs, language_switched, languages, export,
        voice, themes, persona_box, workspace, sessions, model_panel, effort, thinking, thought,
        prepare, on_session, on_page, via, images, to_parent, attached, to_bottom, omitted, summary,
        oobe,
    }
}
