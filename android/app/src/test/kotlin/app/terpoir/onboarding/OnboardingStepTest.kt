// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.onboarding

import org.junit.Assert.assertEquals
import org.junit.Test

class OnboardingStepTest {
    @Test
    fun ageGateComesFirst() {
        assertEquals(OnboardingStep.AGE, onboardingStep(false, ageGatePassed = false, backupPending = false, complete = false))
    }

    @Test
    fun createAfterTheAgeGate() {
        assertEquals(OnboardingStep.CREATE, onboardingStep(false, ageGatePassed = true, backupPending = false, complete = false))
    }

    @Test
    fun backupBlocksEverythingElseOnceTheJournalExists() {
        assertEquals(OnboardingStep.BACKUP, onboardingStep(true, ageGatePassed = true, backupPending = true, complete = false))
    }

    @Test
    fun tourAfterTheBackupIsResolved() {
        assertEquals(OnboardingStep.PRIVACY_TOUR, onboardingStep(true, ageGatePassed = true, backupPending = false, complete = false))
    }

    @Test
    fun doneAfterTheTour() {
        assertEquals(OnboardingStep.DONE, onboardingStep(true, ageGatePassed = true, backupPending = false, complete = true))
    }

    @Test
    fun anExistingJournalSkipsTheAgeGate() {
        // A journal from before onboarding existed: straight to the tour.
        assertEquals(OnboardingStep.PRIVACY_TOUR, onboardingStep(true, ageGatePassed = false, backupPending = false, complete = false))
    }

    @Test
    fun aDeletedJournalStartsAtCreateNotTheAgeGateAgain() {
        assertEquals(OnboardingStep.CREATE, onboardingStep(false, ageGatePassed = true, backupPending = false, complete = true))
    }
}
