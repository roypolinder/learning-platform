//! Account emails in the learner's language. Each email is written once as [`Content`] and
//! rendered twice: as HTML with the platform's look, and as plain text for clients that don't
//! show HTML (spam filters also expect both).
//!
//! Everything interpolated into HTML is escaped, so a display name like `<b>` stays text.

use super::OutgoingEmail;

const APP_NAME: &str = "Learning Platform";
// Design tokens from apps/web/src/routes/layout.css (light theme).
const ACCENT: &str = "#5b5bd6";
const FG: &str = "#18181b";
const FG_MUTED: &str = "#52525b";
const FG_SUBTLE: &str = "#71717a";
const LINE: &str = "#e4e4e7";
const BG: &str = "#f4f4f5";
const FONT: &str = "-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    En,
    Nl,
}

/// Texts of one email. Paragraphs may mark text as bold with `**…**`.
struct Content {
    subject: String,
    /// Shown by mail clients next to the subject in the inbox.
    preview: String,
    heading: String,
    paragraphs: Vec<String>,
    action: Option<Action>,
    /// Small print below the button, e.g. how long the link works.
    note: Option<String>,
}

struct Action {
    label: String,
    url: String,
}

/// Builds the emails for one recipient's language.
pub struct Templates<'a> {
    lang: Lang,
    /// The web app, linked in the footer.
    site: &'a str,
}

impl<'a> Templates<'a> {
    pub fn new(locale: &str, site: &'a str) -> Self {
        let lang = if locale == "nl" { Lang::Nl } else { Lang::En };
        Self { lang, site }
    }

    fn pick(&self, en: &str, nl: &str) -> String {
        match self.lang {
            Lang::En => en.to_owned(),
            Lang::Nl => nl.to_owned(),
        }
    }

    fn greeting(&self, name: &str) -> String {
        match self.lang {
            Lang::En => format!("Hi {name},"),
            Lang::Nl => format!("Hoi {name},"),
        }
    }

    pub fn verify_email(&self, to: &str, name: &str, link: &str) -> OutgoingEmail {
        self.render(
            to,
            Content {
                subject: self.pick("Confirm your email address", "Bevestig je e-mailadres"),
                preview: self.pick(
                    "One click and your account is ready.",
                    "Eén klik en je account is klaar.",
                ),
                heading: self.pick("Confirm your email address", "Bevestig je e-mailadres"),
                paragraphs: vec![
                    self.greeting(name),
                    self.pick(
                        "Welcome to the Learning Platform! Confirm that this is your email address, so you can always get back into your account.",
                        "Welkom bij het Learning Platform! Bevestig dat dit jouw e-mailadres is, zodat je altijd weer in je account kunt.",
                    ),
                ],
                action: Some(Action {
                    label: self.pick("Confirm email address", "E-mailadres bevestigen"),
                    url: link.to_owned(),
                }),
                note: Some(self.pick(
                    "The link works for 48 hours. Didn't create an account? Then you can ignore this email.",
                    "De link werkt 48 uur. Heb je geen account aangemaakt? Dan kun je deze e-mail negeren.",
                )),
            },
        )
    }

    pub fn reset_password(&self, to: &str, name: &str, link: &str) -> OutgoingEmail {
        self.render(
            to,
            Content {
                subject: self.pick("Reset your password", "Stel je wachtwoord opnieuw in"),
                preview: self.pick(
                    "Choose a new password for your account.",
                    "Kies een nieuw wachtwoord voor je account.",
                ),
                heading: self.pick("Choose a new password", "Kies een nieuw wachtwoord"),
                paragraphs: vec![
                    self.greeting(name),
                    self.pick(
                        "Someone asked to reset the password of your account. If that was you, choose a new password below.",
                        "Iemand heeft gevraagd om het wachtwoord van je account opnieuw in te stellen. Was jij dat? Kies dan hieronder een nieuw wachtwoord.",
                    ),
                ],
                action: Some(Action {
                    label: self.pick("Choose a new password", "Nieuw wachtwoord kiezen"),
                    url: link.to_owned(),
                }),
                note: Some(self.pick(
                    "The link works for 1 hour and only once. Didn't ask for this? Then ignore this email; your password stays the same.",
                    "De link werkt 1 uur en maar één keer. Heb je dit niet gevraagd? Negeer deze e-mail dan; je wachtwoord blijft hetzelfde.",
                )),
            },
        )
    }

    /// Sent to the new address; the change happens only when this link is opened.
    pub fn confirm_new_email(&self, to: &str, name: &str, link: &str) -> OutgoingEmail {
        self.render(
            to,
            Content {
                subject: self.pick("Confirm your new email address", "Bevestig je nieuwe e-mailadres"),
                preview: self.pick(
                    "Confirm the new address for your account.",
                    "Bevestig het nieuwe adres voor je account.",
                ),
                heading: self.pick("Confirm your new email address", "Bevestig je nieuwe e-mailadres"),
                paragraphs: vec![
                    self.greeting(name),
                    match self.lang {
                        Lang::En => format!("You asked to use **{to}** for your account from now on. Confirm it below."),
                        Lang::Nl => format!("Je hebt gevraagd om voortaan **{to}** voor je account te gebruiken. Bevestig het hieronder."),
                    },
                ],
                action: Some(Action {
                    label: self.pick("Confirm new address", "Nieuw adres bevestigen"),
                    url: link.to_owned(),
                }),
                note: Some(self.pick(
                    "The link works for 24 hours. Until then your account keeps its current address.",
                    "De link werkt 24 uur. Tot die tijd houdt je account het huidige adres.",
                )),
            },
        )
    }

    /// Sent to the current address, so a hijacked session cannot move the account silently.
    pub fn email_change_requested(&self, to: &str, name: &str, new_email: &str) -> OutgoingEmail {
        self.render(
            to,
            Content {
                subject: self.pick("Your email address is being changed", "Je e-mailadres wordt gewijzigd"),
                preview: self.pick(
                    "Wasn't this you? Cancel the change in Settings.",
                    "Was jij dit niet? Annuleer de wijziging bij Instellingen.",
                ),
                heading: self.pick("Your email address is being changed", "Je e-mailadres wordt gewijzigd"),
                paragraphs: vec![
                    self.greeting(name),
                    match self.lang {
                        Lang::En => format!("Someone asked to change the email address of your account to **{new_email}**. It changes once that address is confirmed."),
                        Lang::Nl => format!("Iemand heeft gevraagd om het e-mailadres van je account te wijzigen in **{new_email}**. Het verandert zodra dat adres is bevestigd."),
                    },
                    self.pick(
                        "Wasn't this you? Sign in, cancel the change in Settings and change your password.",
                        "Was jij dit niet? Log in, annuleer de wijziging bij Instellingen en wijzig je wachtwoord.",
                    ),
                ],
                action: Some(Action {
                    label: self.pick("Open Settings", "Naar Instellingen"),
                    url: format!("{}/settings", self.site),
                }),
                note: None,
            },
        )
    }

    pub fn password_changed(&self, to: &str, name: &str) -> OutgoingEmail {
        self.render(
            to,
            Content {
                subject: self.pick("Your password was changed", "Je wachtwoord is gewijzigd"),
                preview: self.pick(
                    "Other devices were signed out.",
                    "Andere apparaten zijn uitgelogd.",
                ),
                heading: self.pick("Your password was changed", "Je wachtwoord is gewijzigd"),
                paragraphs: vec![
                    self.greeting(name),
                    self.pick(
                        "The password of your account was just changed, and your other devices were signed out.",
                        "Het wachtwoord van je account is zojuist gewijzigd, en je andere apparaten zijn uitgelogd.",
                    ),
                    self.pick(
                        "Wasn't this you? Reset your password right away.",
                        "Was jij dit niet? Stel je wachtwoord dan meteen opnieuw in.",
                    ),
                ],
                action: Some(Action {
                    label: self.pick("Reset password", "Wachtwoord opnieuw instellen"),
                    url: format!("{}/forgot-password", self.site),
                }),
                note: None,
            },
        )
    }

    pub fn account_deleted(&self, to: &str, name: &str) -> OutgoingEmail {
        self.render(
            to,
            Content {
                subject: self.pick("Your account was deleted", "Je account is verwijderd"),
                preview: self.pick(
                    "Everything in your account has been removed.",
                    "Alles in je account is verwijderd.",
                ),
                heading: self.pick("Your account was deleted", "Je account is verwijderd"),
                paragraphs: vec![
                    self.greeting(name),
                    self.pick(
                        "Your account and everything in it (progress, notes, project and agent connections) have been deleted. You won't get any more emails from us.",
                        "Je account en alles daarin (voortgang, notities, project en agentkoppelingen) zijn verwijderd. Je krijgt geen e-mails meer van ons.",
                    ),
                    self.pick(
                        "Thanks for learning with us. You're always welcome back.",
                        "Bedankt dat je met ons hebt geleerd. Je bent altijd welkom terug.",
                    ),
                ],
                action: None,
                note: None,
            },
        )
    }

    /// Sent from the admin log page to check the mail configuration.
    pub fn test_email(&self, to: &str, name: &str) -> OutgoingEmail {
        self.render(
            to,
            Content {
                subject: "Learning Platform email check".into(),
                preview: "Account emails are working.".into(),
                heading: "Account emails are working".into(),
                paragraphs: vec![
                    self.greeting(name),
                    "You sent this message from the log page. It arrived, so the mail server accepts account emails from the platform.".into(),
                ],
                action: Some(Action {
                    label: "Open the log".into(),
                    url: format!("{}/admin/logs", self.site),
                }),
                note: None,
            },
        )
    }

    fn render(&self, to: &str, content: Content) -> OutgoingEmail {
        OutgoingEmail {
            to: to.to_owned(),
            subject: content.subject.clone(),
            body: self.text(&content),
            html: Some(self.html(&content)),
        }
    }

    fn footer(&self) -> String {
        self.pick(
            "You get this email because of your account on the Learning Platform.",
            "Je krijgt deze e-mail vanwege je account op het Learning Platform.",
        )
    }

    fn text(&self, content: &Content) -> String {
        let mut out = String::new();
        for paragraph in &content.paragraphs {
            out.push_str(&paragraph.replace("**", ""));
            out.push_str("\n\n");
        }
        if let Some(action) = &content.action {
            out.push_str(&format!("{}:\n{}\n\n", action.label, action.url));
        }
        if let Some(note) = &content.note {
            out.push_str(note);
            out.push_str("\n\n");
        }
        out.push_str(&format!(
            "— {APP_NAME}\n\n{}\n{}\n",
            self.footer(),
            self.site
        ));
        out
    }

    fn html(&self, content: &Content) -> String {
        let lang = match self.lang {
            Lang::En => "en",
            Lang::Nl => "nl",
        };
        let paragraphs: String = content
            .paragraphs
            .iter()
            .map(|paragraph| {
                format!(
                    r#"<p style="margin:0 0 16px;font-size:15px;line-height:1.6;color:{FG_MUTED};">{}</p>"#,
                    bold(&escape(paragraph))
                )
            })
            .collect();
        let action = content.action.as_ref().map_or_else(String::new, |action| {
            let url = escape(&action.url);
            let fallback = self.pick(
                "Button not working? Copy this link into your browser:",
                "Werkt de knop niet? Kopieer deze link naar je browser:",
            );
            format!(
                r#"<table role="presentation" cellpadding="0" cellspacing="0" border="0" style="margin:8px 0 24px;"><tr><td style="border-radius:6px;background:{ACCENT};"><a href="{url}" style="display:inline-block;padding:12px 22px;font-family:{FONT};font-size:15px;font-weight:600;color:#ffffff;text-decoration:none;border-radius:6px;">{label}</a></td></tr></table>
<p style="margin:0 0 4px;font-size:12px;line-height:1.5;color:{FG_SUBTLE};">{fallback}</p>
<p style="margin:0 0 20px;font-size:12px;line-height:1.5;word-break:break-all;"><a href="{url}" style="color:{ACCENT};">{url}</a></p>"#,
                label = escape(&action.label),
                fallback = escape(&fallback),
            )
        });
        let note = content.note.as_ref().map_or_else(String::new, |note| {
            format!(
                r#"<p style="margin:0;padding-top:16px;border-top:1px solid {LINE};font-size:13px;line-height:1.5;color:{FG_SUBTLE};">{}</p>"#,
                escape(note)
            )
        });
        let site = escape(self.site);
        let site_label = escape(
            self.site
                .trim_start_matches("https://")
                .trim_start_matches("http://"),
        );
        format!(
            r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="color-scheme" content="light">
<meta name="supported-color-schemes" content="light">
<title>{subject}</title>
</head>
<body style="margin:0;padding:0;background:{BG};">
<div style="display:none;max-height:0;overflow:hidden;opacity:0;">{preview}</div>
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="background:{BG};">
<tr><td align="center" style="padding:32px 16px;">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="max-width:520px;font-family:{FONT};">
<tr><td style="padding:0 4px 16px;">
<a href="{site}" style="text-decoration:none;color:{FG};font-size:15px;font-weight:600;"><span style="display:inline-block;width:12px;height:12px;border-radius:3px;background:{ACCENT};vertical-align:-1px;margin-right:8px;"></span>{APP_NAME}</a>
</td></tr>
<tr><td style="background:#ffffff;border:1px solid {LINE};border-radius:10px;padding:32px;">
<h1 style="margin:0 0 20px;font-size:20px;line-height:1.3;font-weight:600;color:{FG};">{heading}</h1>
{paragraphs}{action}{note}
</td></tr>
<tr><td style="padding:16px 4px 0;font-size:12px;line-height:1.5;color:{FG_SUBTLE};">
{footer}<br><a href="{site}" style="color:{FG_SUBTLE};">{site_label}</a>
</td></tr>
</table>
</td></tr>
</table>
</body>
</html>
"#,
            subject = escape(&content.subject),
            preview = escape(&content.preview),
            heading = escape(&content.heading),
            footer = escape(&self.footer()),
        )
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// Turns `**text**` into bold. Runs on escaped text, so it cannot introduce other markup.
fn bold(escaped: &str) -> String {
    let mut out = String::new();
    for (index, part) in escaped.split("**").enumerate() {
        if index % 2 == 1 {
            out.push_str(&format!(r#"<strong style="color:{FG};">{part}</strong>"#));
        } else {
            out.push_str(part);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_html_and_text_with_the_link() {
        let mail = Templates::new("nl", "https://learn.example.nl").reset_password(
            "ada@example.com",
            "Ada",
            "https://learn.example.nl/reset-password#token=abc",
        );
        assert_eq!(mail.subject, "Stel je wachtwoord opnieuw in");
        assert!(
            mail.body
                .contains("https://learn.example.nl/reset-password#token=abc")
        );
        let html = mail.html.unwrap();
        assert!(html.contains(r#"<html lang="nl">"#));
        assert!(html.contains(r#"href="https://learn.example.nl/reset-password#token=abc""#));
        assert!(html.contains("Nieuw wachtwoord kiezen"));
    }

    #[test]
    fn escapes_names_in_html() {
        let mail = Templates::new("en", "https://learn.example.nl").email_change_requested(
            "ada@example.com",
            "<script>alert(1)</script>",
            "new**@example.com",
        );
        let html = mail.html.unwrap();
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(
            mail.body.contains("<script>alert(1)</script>"),
            "text stays as typed"
        );
    }
}

#[cfg(test)]
mod preview {
    /// Writes every email as HTML to LP_EMAIL_PREVIEW_DIR, for looking at them in a browser.
    #[test]
    fn write_previews() {
        let Ok(dir) = std::env::var("LP_EMAIL_PREVIEW_DIR") else {
            return;
        };
        let site = "https://learn.polinderit.nl";
        let link = "https://learn.polinderit.nl/verify-email#token=lp_mail_example";
        for locale in ["en", "nl"] {
            let t = super::Templates::new(locale, site);
            for (name, mail) in [
                ("verify", t.verify_email("ada@example.com", "Ada", link)),
                ("reset", t.reset_password("ada@example.com", "Ada", link)),
                (
                    "confirm-new",
                    t.confirm_new_email("ada.new@example.com", "Ada", link),
                ),
                (
                    "change-requested",
                    t.email_change_requested("ada@example.com", "Ada", "ada.new@example.com"),
                ),
                (
                    "password-changed",
                    t.password_changed("ada@example.com", "Ada"),
                ),
                ("deleted", t.account_deleted("ada@example.com", "Ada")),
                ("test", t.test_email("ada@example.com", "Ada")),
            ] {
                std::fs::write(format!("{dir}/{locale}-{name}.html"), mail.html.unwrap()).unwrap();
                std::fs::write(format!("{dir}/{locale}-{name}.txt"), mail.body).unwrap();
            }
        }
    }
}
