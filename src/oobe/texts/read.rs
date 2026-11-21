//! 引导上的字照哪几个键读（施工 LW-9，`from_json`）：放一起，`texts.rs` 不超过 500 行。

use super::{
    Controls, Currency, Done, Heading, Icons, Keys, Language, Model, Persona, Preset, Profile,
    Semantic, SemanticDownload, Texts, Theme, Welcome,
};

crate::from_json::record! { Welcome, deny { title, sub, start } }

crate::from_json::record! {
    Keys, deny {
        pick, form, driver, test, anyway, editing, searching, models, persona, edit, writing,
        custom, custom_name, create, view, enter_edit, enter_pick, presets, more, more_vim, theme,
    }
}

crate::from_json::record! { Heading, deny { title, sub } }

crate::from_json::record! { Language, deny { auto } flatten head }

crate::from_json::record! { Icons, deny { yes, no } flatten head }

crate::from_json::record! { Controls, deny { arrows, arrows_note, vim, vim_note } flatten head }

crate::from_json::record! { Currency { default, names, other } flatten head }

crate::from_json::record! {
    Profile, deny {
        fields, hints, other, next, skip, saving,
    }
    flatten head
}

crate::from_json::record! {
    Theme, deny {
        rows, modes, colors, from_avatar, from_seed, no_avatar, from_wallpaper, no_wallpaper,
        mascot, mascot_note, ground, ground_note, seed_default, next, saving,
    }
    flatten head
}

crate::from_json::record! { Semantic, deny { on, off, download } flatten head }

crate::from_json::record! {
    SemanticDownload, deny {
        size, preparing, progress, verifying, installing, retry, skip,
    }
}

crate::from_json::record! {
    Model, deny {
        loading, found, configured, local, other, more, more_title, more_search, no_match,
        search_cue, custom, custom_note, ready_title, ready, keep, change, key, key_env, key_none,
        key_needed, id, id_bad, id_taken, save_anyway, model_for_save, cost, key_sub, custom_head,
        url, name, name_hint, url_bad, driver, drivers, testing, connected, connected_request,
        pick_sub, no_list, model_name, model_example, failed, stages, search, test, more_above,
        more_below, saving,
    }
    flatten head
}

crate::from_json::record! {
    Persona, deny {
        fields, summary, summary_hint, hints, examples_some, next, skip, name_needed, existing,
        saving, avatar, avatar_hint, avatar_set,
    }
    flatten head
}

crate::from_json::record! {
    Preset, deny {
        custom, custom_note, name, name_hint, name_needed, loading, create, view_hint,
    }
    flatten head
}

crate::from_json::record! { Done, deny { title, summary, start } }

crate::from_json::record! {
    Texts, deny {
        welcome, steps, keys, language, icons, controls, currency, model, persona, theme, profile,
        preset, semantic, done, offline, unmarked,
    }
}
