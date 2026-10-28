<script setup lang="ts">
// Filiz — KONSEPT çizim, nihai değil (design README "Fidelity").
// Geometri, design_files/Kindora Phase 1 - Visual Foundation.dc.html içindeki
// `mascot()` işlevinden birebir aktarıldı; yeni karakter veya durum eklenmedi.
// Son illüstrasyon mascot_brief/ sürecinden gelecek.
import { computed } from "vue";
import type { MascotPose } from "../agent/presentation";

const props = withDefaults(defineProps<{ pose: MascotPose; size?: number; motion?: boolean }>(), {
  size: 120,
  motion: true,
});

const body = "#5265D8";
const ink = "#1B2150";
const sleepy = computed(() => props.pose === "sleeping");
const joy = computed(() => props.pose === "happy" || props.pose === "greeting");
const closed = computed(() => sleepy.value);
const arms = computed(() => {
  switch (props.pose) {
    case "greeting":
      return ["M27 80 Q19 86 21 93", "M93 70 Q106 60 104 46"];
    case "happy":
      return ["M29 64 Q17 52 21 40", "M91 64 Q103 52 99 40"];
    case "permission":
      return ["M30 86 Q38 96 48 96", "M90 86 Q82 96 72 96"];
    default:
      return ["M27 80 Q19 86 21 93", "M93 80 Q101 86 99 93"];
  }
});
const eyeDx = computed(() => (props.pose === "thinking" ? 2 : 0));
const eyeDy = computed(() => (props.pose === "thinking" ? -3 : 0));
const eyeRy = computed(() => (props.pose === "thinking" ? 4 : 4.8));
</script>

<template>
  <div
    class="kd-mascot"
    :class="{ 'kd-mascot--bob': motion && !sleepy }"
    :style="{ width: `${String(size)}px`, height: `${String(size)}px` }"
  >
    <svg viewBox="0 0 120 120" :width="size" :height="size" aria-hidden="true">
      <ellipse cx="60" cy="112" rx="26" ry="4" fill="rgba(26,32,51,.14)" />
      <template v-if="sleepy">
        <path
          d="M60 37 Q60 30 65 27"
          stroke="#21A08B"
          stroke-width="3"
          fill="none"
          stroke-linecap="round"
        />
        <path d="M65 27 C71 22 78 25 79 27 C75 33 68 32 65 27Z" fill="#2BBFA7" />
      </template>
      <template v-else>
        <path
          d="M60 37 L60 26"
          stroke="#21A08B"
          stroke-width="3"
          fill="none"
          stroke-linecap="round"
        />
        <path d="M60 28 C53 18 44 20 41 21 C43 30 53 32 60 28Z" fill="#2BBFA7" />
        <path d="M60 26 C66 15 76 15 79 16 C78 26 68 30 60 26Z" fill="#3ED0B8" />
      </template>
      <path
        v-for="d in arms"
        :key="d"
        :d="d"
        :stroke="body"
        stroke-width="8"
        fill="none"
        stroke-linecap="round"
      />
      <path
        d="M60 36 C85 36 97 55 97 74 C97 95 81 105 60 105 C39 105 23 95 23 74 C23 55 35 36 60 36Z"
        :fill="body"
      />
      <path
        d="M42 46 C48 41 54 40 58 40"
        stroke="rgba(255,255,255,.35)"
        stroke-width="3"
        fill="none"
        stroke-linecap="round"
      />
      <rect x="34" y="56" width="52" height="36" rx="18" fill="#EEF0FC" />
      <!-- gözler -->
      <template v-if="joy">
        <path
          d="M46 74 Q50 68 54 74"
          :stroke="ink"
          stroke-width="3"
          fill="none"
          stroke-linecap="round"
        />
        <path
          d="M66 74 Q70 68 74 74"
          :stroke="ink"
          stroke-width="3"
          fill="none"
          stroke-linecap="round"
        />
      </template>
      <template v-else-if="closed">
        <path
          d="M46 72 Q50 76 54 72"
          :stroke="ink"
          stroke-width="3"
          fill="none"
          stroke-linecap="round"
        />
        <path
          d="M66 72 Q70 76 74 72"
          :stroke="ink"
          stroke-width="3"
          fill="none"
          stroke-linecap="round"
        />
      </template>
      <g v-else :class="{ 'kd-mascot__blink': motion }">
        <ellipse :cx="50 + eyeDx" :cy="72 + eyeDy" rx="3.6" :ry="eyeRy" :fill="ink" />
        <ellipse :cx="70 + eyeDx" :cy="72 + eyeDy" rx="3.6" :ry="eyeRy" :fill="ink" />
      </g>
      <template v-if="!sleepy">
        <circle cx="42" cy="81" r="3.5" fill="#F4A6B6" opacity=".8" />
        <circle cx="78" cy="81" r="3.5" fill="#F4A6B6" opacity=".8" />
      </template>
      <!-- ağız -->
      <path v-if="joy" d="M53 80 Q60 89 67 80 Z" :fill="ink" />
      <ellipse v-else-if="pose === 'permission'" cx="60" cy="83" rx="2.6" ry="3" :fill="ink" />
      <path
        v-else-if="sleepy"
        d="M57 83 L63 83"
        :stroke="ink"
        stroke-width="3"
        fill="none"
        stroke-linecap="round"
      />
      <path
        v-else-if="pose === 'thinking'"
        d="M56 83 Q60 81 64 83"
        :stroke="ink"
        stroke-width="3"
        fill="none"
        stroke-linecap="round"
      />
      <path
        v-else
        d="M55 81 Q60 85 65 81"
        :stroke="ink"
        stroke-width="3"
        fill="none"
        stroke-linecap="round"
      />
      <!-- durum işaretleri -->
      <g v-if="pose === 'permission'">
        <circle cx="99" cy="40" r="11" fill="#8C80E8" stroke="#FFFFFF" stroke-width="2.5" />
        <path
          d="M95.5 34 L102.5 34 L95.5 46 L102.5 46 Z"
          stroke="#FFFFFF"
          stroke-width="2"
          fill="none"
          stroke-linejoin="round"
        />
      </g>
      <template v-if="pose === 'thinking'">
        <circle cx="93" cy="46" r="2.5" fill="#B6AEF2" />
        <circle cx="100" cy="38" r="3.3" fill="#B6AEF2" />
        <circle cx="108" cy="29" r="4.2" fill="#B6AEF2" />
      </template>
      <template v-if="sleepy">
        <text
          x="90"
          y="44"
          font-size="15"
          font-weight="700"
          fill="#8C80E8"
          font-family="Fredoka,sans-serif"
        >
          z
        </text>
        <text
          x="101"
          y="32"
          font-size="11"
          font-weight="700"
          fill="#B6AEF2"
          font-family="Fredoka,sans-serif"
        >
          z
        </text>
      </template>
      <template v-if="pose === 'happy'">
        <path d="M18 30 L20 36 L26 38 L20 40 L18 46 L16 40 L10 38 L16 36 Z" fill="#E7A83E" />
        <path
          d="M104 22 L105.5 26 L109.5 27.5 L105.5 29 L104 33 L102.5 29 L98.5 27.5 L102.5 26 Z"
          fill="#2BBFA7"
        />
      </template>
    </svg>
  </div>
</template>

<style scoped>
.kd-mascot {
  flex-shrink: 0;
}
.kd-mascot--bob {
  animation: kd-bob 3.4s ease-in-out infinite;
}
.kd-mascot__blink {
  transform-origin: 60px 72px;
  animation: kd-blink 5s infinite;
}
@keyframes kd-bob {
  0%,
  100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-4px);
  }
}
@keyframes kd-blink {
  0%,
  92%,
  100% {
    transform: scaleY(1);
  }
  95% {
    transform: scaleY(0.1);
  }
}
:global([data-kd-motion="reduced"]) .kd-mascot--bob,
:global([data-kd-motion="reduced"]) .kd-mascot__blink {
  animation: none;
}
</style>
