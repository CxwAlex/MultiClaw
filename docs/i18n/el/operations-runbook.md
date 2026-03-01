# Εγχειρίδιο Λειτουργιών MultiClaw (Operations Runbook)

Αυτό το εγχειρίδιο προορίζεται για τους διαχειριστές του συστήματος που είναι υπεύθυνοι για τη διαθεσιμότητα, την ασφάλεια και την απόκριση σε περιστατικά.

Τελευταία επαλήθευση: **18 Φεβρουαρίου 2026**.

## Πεδίο Εφαρμογής

Το έγγραφο καλύπτει τις καθημερινές λειτουργίες (Day-2 operations):
- Εκκίνηση και επίβλεψη του runtime.
- Ελέγχους καλής λειτουργίας (health checks) και διαγνωστικά.
- Διαδικασίες ασφαλούς ανάπτυξης (rollout) και επαναφοράς (rollback).
- Διαλογή (triage) και αποκατάσταση μετά από περιστατικά.

Για την αρχική εγκατάσταση, ανατρέξτε στον οδηγό [one-click-bootstrap.md](one-click-bootstrap.md).

## Λειτουργίες Χρόνου Εκτέλεσης (Runtime Modes)

| Λειτουργία | Εντολή | Χρήση |
|:---|:---|:---|
| Προσκήνιο (Foreground) | `multiclaw daemon` | Τοπική αποσφαλμάτωση και δοκιμές. |
| Πύλη (Gateway) | `multiclaw gateway` | Έλεγχος τελικών σημείων (endpoints) webhook. |
| Υπηρεσία Συστήματος | `multiclaw service install && multiclaw service start` | Μόνιμη εκτέλεση υπό τη διαχείριση του συστήματος. |

## Βασική Ροή Εργασίας Διαχειριστή

1. **Επικύρωση Ρυθμίσεων**:
   ```bash
   multiclaw status
   ```
2. **Διαγνωστικός Έλεγχος**:
   ```bash
   multiclaw doctor
   multiclaw channel doctor
   ```
3. **Εκκίνηση Daemon**:
   ```bash
   multiclaw daemon
   ```
4. **Διαχείριση Υπηρεσίας**:
   ```bash
   multiclaw service install
   multiclaw service start
   multiclaw service status
   ```

## Δείκτες Κατάστασης και Υγείας

| Δείκτης | Εντολή / Αρχείο | Αναμενόμενη Κατάσταση |
|:---|:---|:---|
| Εγκυρότητα Ρυθμίσεων | `multiclaw doctor` | Επιτυχής έλεγχος χωρίς κρίσιμα σφάλματα. |
| Συνδεσιμότητα Καναλιών | `multiclaw channel doctor` | Όλα τα ενεργά κανάλια είναι online. |
| Σύνοψη Runtime | `multiclaw status` | Εμφάνιση σωστών παρόχων και μοντέλων. |
| Daemon Heartbeat | `~/.multiclaw/daemon_state.json` | Το αρχείο ενημερώνεται σε πραγματικό χρόνο. |

## Καταγραφές (Logs) και Διαγνωστικά

- **macOS / Windows**:
  - `~/.multiclaw/logs/daemon.stdout.log`
  - `~/.multiclaw/logs/daemon.stderr.log`
- **Linux (systemd)**:
  ```bash
  journalctl --user -u multiclaw.service -f
  ```

## Διαλογή Περιστατικών (Incident Triage)

Σε περίπτωση δυσλειτουργίας, ακολουθήστε τα παρακάτω βήματα:

1. **Ανάλυση Κατάστασης**:
   ```bash
   multiclaw status
   multiclaw doctor
   multiclaw channel doctor
   ```
2. **Έλεγχος Υπηρεσίας**:
   ```bash
   multiclaw service status
   ```
3. **Επανεκκίνηση**:
   Εάν η υπηρεσία δεν αποκρίνεται, πραγματοποιήστε καθαρή επανεκκίνηση:
   ```bash
   multiclaw service stop
   multiclaw service start
   ```
4. **Έλεγχος Διαπιστευτηρίων**:
   Επαληθεύστε τα API keys και τις λίστες επιτρεπόμενων χρηστών στο `~/.multiclaw/config.toml`.
5. **Έλεγχος Πύλης**:
   Επαληθεύστε τις ρυθμίσεις σύνδεσης στην ενότητα `[gateway]` και την τοπική συνδεσιμότητα.

## Διαδικασία Αλλαγών (Safe Change Management)

1. Δημιουργήστε αντίγραφο ασφαλείας του `config.toml`.
2. Εφαρμόστε μία αλλαγή τη φορά.
3. Εκτελέστε την εντολή `multiclaw doctor` για επικύρωση.
4. Επανεκκινήστε την υπηρεσία.
5. Επαληθεύστε τη λειτουργία μέσω των εντολών `status` και `channel doctor`.

## Διαδικασία Επαναφοράς (Rollback)

Εάν παρατηρηθεί υποβάθμιση της υπηρεσίας μετά από αλλαγή:
1. Επαναφέρετε το προηγούμενο έγκυρο αρχείο `config.toml`.
2. Επανεκκινήστε το runtime (`daemon` ή `service`).
3. Επιβεβαιώστε την αποκατάσταση με τους ελέγχους `doctor` και `channel doctor`.
4. Αναλύστε την αιτία του προβλήματος πριν από νέα προσπάθεια αλλαγής.

## Σχετική Τεκμηρίωση

- [one-click-bootstrap.md](one-click-bootstrap.md)
- [troubleshooting.md](troubleshooting.md)
- [config-reference.md](config-reference.md)
- [commands-reference.md](commands-reference.md)
