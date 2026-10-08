//! PROPOSED private Desktop module; enabled only by the supplemental assertion feature.
//! No new public API. Wiring and feature registration require Root's separate freeze/review.
use super::super::native_preview::AttachmentLife;
use super::super::native_save::{DocumentLife, DocumentRow, IntentLife};
use super::super::{NativeEditorObservation, NativeFrameObservation};
use std::sync::Arc;

pub(super) struct OwnedCheckpoint {
    view: NativeEditorObservation,
    anchor: Option<usize>,
    preferred: Option<usize>,
    frame: Option<NativeFrameObservation>,
    key_sequence: u64,
    focus_epoch: u64,
    last_intent: Option<u64>,
    queued_intent: Option<Arc<IntentLife>>,
    life: Arc<DocumentLife>,
    attachment: Arc<AttachmentLife>,
    object_id: u64,
}
impl OwnedCheckpoint {
    // Called inside the genuine owner AFTER input admission and BEFORE scratch edit.
    pub(super) fn capture(row: &DocumentRow) -> Self {
        Self {
            view: row.view.clone(),
            anchor: row.selection_anchor,
            preferred: row.preferred_column,
            frame: row.frame.clone(),
            key_sequence: row.key_sequence,
            focus_epoch: row.focus_epoch,
            last_intent: row.last_intent_id,
            queued_intent: row.queued_intent.clone(),
            life: row.life.clone(),
            attachment: row.attachment.clone(),
            object_id: row.object_id,
        }
    }
    pub(super) fn assert_max_rejection(&self, row: &DocumentRow) {
        assert_eq!(self.view.bytes, b"abcd\nx\nabcdef");
        assert_eq!(
            (
                self.view.cursor,
                self.view.selection,
                self.anchor,
                self.preferred
            ),
            (6, Some((6, 11)), Some(11), Some(4))
        );
        assert!(matches!(
            self.view.phase,
            super::super::NativeEditorPhase::Loaded
        ));
        assert!(self.queued_intent.is_none() && self.last_intent.is_none());
        assert!(self.view.text_generation == u64::MAX || self.view.view_version == u64::MAX);
        let frame = self
            .frame
            .as_ref()
            .expect("actual owner render before rejection");
        assert_eq!(
            (frame.text_generation, frame.view_version),
            (self.view.text_generation, self.view.view_version)
        );
        assert_eq!(row.view, self.view);
        assert_eq!(
            (row.selection_anchor, row.preferred_column),
            (self.anchor, self.preferred)
        );
        assert_eq!(row.frame, self.frame);
        assert_eq!(
            (row.key_sequence, row.focus_epoch, row.last_intent_id),
            (self.key_sequence, self.focus_epoch, self.last_intent)
        );
        assert!(Arc::ptr_eq(&row.life, &self.life));
        assert!(Arc::ptr_eq(&row.attachment, &self.attachment));
        assert_eq!(row.object_id, self.object_id);
        match (&row.queued_intent, &self.queued_intent) {
            (None, None) => {}
            (Some(a), Some(b)) => assert!(Arc::ptr_eq(a, b)),
            _ => panic!("denied edit changed original intent ownership"),
        }
        // Diagnostics prove this assertion body was reached, never grant authority.
        if self.view.text_generation == u64::MAX {
            eprintln!("ADAPTER_ASSERT actual-text-max hidden-owner-atomic PASS");
        } else {
            eprintln!("ADAPTER_ASSERT actual-view-max hidden-owner-atomic PASS");
        }
    }
}

// Called with the actual host State and native Gate held in their existing order.
// This finite test-data operation cannot create an editor, frame or grant.
pub(super) fn seed_actual_counter(
    state: &super::State,
    gate: &mut super::super::native_authority::GateState,
    counter: super::super::NativeSaveCounter,
) -> Result<(), super::Status> {
    use super::super::{NativeEditorPhase, NativeSaveCounter};
    use super::Status;
    let save = gate.save.as_ref().ok_or(Status::Denied)?;
    if save.documents.len() != 1 {
        return Err(Status::NotReady);
    }
    let (&id, row) = save.documents.iter().next().ok_or(Status::NotReady)?;
    let actor = &row.view.actor;
    let preview = gate.preview.as_ref().ok_or(Status::Denied)?;
    preview.admit_instance(actor.instance_id)?;
    let attachment = preview.row(&row.attachment)?;
    let instance = state
        .instances
        .get(&actor.instance_id)
        .ok_or(Status::Stale)?;
    let session = state.sessions.get(&actor.session_id).ok_or(Status::Stale)?;
    if attachment.view.actor != *actor
        || !attachment.delivered
        || instance.state != super::InstanceState::Running
        || instance.session != actor.session_id
        || instance.generation != actor.instance_generation
        || gate.now_ms >= instance.expires
        || session.generation != actor.session_generation
        || session.focused != Some(actor.instance_id)
        || session.focus != row.focus_epoch
        || !session.keys.is_empty()
        || !session.pressed.is_empty()
        || session.modifiers != 0
        || session.reset_pending
        || state.pending_frames != 0
        || Arc::strong_count(&row.life) < 2
        || row.life.id != id
        || row.view.document_id != id
        || row.view.bytes != b"abcd\nx\nabcdef"
        || (
            row.view.cursor,
            row.view.selection,
            row.view.first_visible_line,
        ) != (6, Some((6, 11)), 0)
        || (row.selection_anchor, row.preferred_column) != (Some(11), Some(4))
        || (row.view.text_generation, row.view.view_version) != (1, 7)
        || row.view.phase != NativeEditorPhase::Loaded
        || row.queued_intent.is_some()
        || row.last_intent_id.is_some()
        || save.text_exhausted
        || save.view_exhausted
        || save.armed.is_some()
        || !save.allocations.is_empty()
        || save.intents.values().any(|w| w.strong_count() != 0)
        || save.sources.values().any(|w| w.strong_count() != 0)
        || save.originals.values().any(|w| w.strong_count() != 0)
        || save.unresolved_document(id)?
        || save.calls.values().any(|call| {
            call.life.upgrade().is_some_and(|life| {
                !call.claimed || life.view.request.msg_type != 1 || life.view.actor != *actor
            })
        })
    {
        return Err(Status::NotReady);
    }
    // Every fallible check completes before the single counter-field write.
    let row = gate
        .save
        .as_mut()
        .ok_or(Status::Denied)?
        .documents
        .get_mut(&id)
        .ok_or(Status::Stale)?;
    match counter {
        NativeSaveCounter::TextGeneration => row.view.text_generation = u64::MAX,
        NativeSaveCounter::ViewVersion => row.view.view_version = u64::MAX,
        _ => return Err(Status::Invalid),
    }
    Ok(())
}
