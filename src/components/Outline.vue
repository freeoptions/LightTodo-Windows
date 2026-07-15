<script setup lang="ts">
import { computed, ref } from 'vue';
import type { TodoItem as TodoItemType } from '../types/todo';

const props = defineProps<{
  todos: TodoItemType[];
}>();

const parentTodos = computed(() => {
  const todos = props.todos.filter(todo =>
    !todo.disabled &&
    !todo.parentId &&
    todo.subtodos &&
    todo.subtodos.some(subtodo => !subtodo.disabled)
  );
  return todos.sort((a, b) => {
    if (a.completed === b.completed) return 0;
    return a.completed ? 1 : -1;
  });
});

const activeTodoId = ref<string | null>(null);
let activeTimer: ReturnType<typeof window.setTimeout> | null = null;

const revealOutlineItem = (item: HTMLElement) => {
  const list = item.closest('.outline-list') as HTMLElement | null;
  if (!list) return;

  const listRect = list.getBoundingClientRect();
  const itemRect = item.getBoundingClientRect();
  const padding = 12;
  let nextLeft = list.scrollLeft;

  if (itemRect.left < listRect.left + padding) {
    nextLeft -= listRect.left + padding - itemRect.left;
  } else if (itemRect.right > listRect.right - padding) {
    nextLeft += itemRect.right - (listRect.right - padding);
  }

  if (nextLeft !== list.scrollLeft) {
    list.scrollTo({
      left: Math.max(0, nextLeft),
      behavior: 'smooth'
    });
  }
};

const scrollToTodo = (id: string, event?: MouseEvent) => {
  const outlineItem = event?.currentTarget as HTMLElement | null;
  if (outlineItem) revealOutlineItem(outlineItem);

  activeTodoId.value = id;
  if (activeTimer) window.clearTimeout(activeTimer);
  activeTimer = window.setTimeout(() => {
    activeTodoId.value = null;
  }, 900);

  const element = document.getElementById(`todo-${id}`);
  if (element) {
    const outlineContainer = document.querySelector('.outline-container') as HTMLElement;
    const outlineHeight = outlineContainer ? outlineContainer.offsetHeight : 0;
    const festivalBanner = document.querySelector('.festival-banner') as HTMLElement;
    const festivalHeight = festivalBanner ? festivalBanner.offsetHeight : 0;
    const offset = outlineHeight + festivalHeight + 20;
    const elementRect = element.getBoundingClientRect();
    const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
    const targetPosition = scrollTop + elementRect.top - offset;

    window.scrollTo({
      top: targetPosition,
      behavior: 'smooth'
    });
  }
};

const handleWheel = (e: WheelEvent) => {
  const target = e.currentTarget as HTMLElement;
  target.scrollLeft += e.deltaY;
};
</script>

<template>
  <div class="outline-container" v-if="parentTodos.length > 0">
    <div class="outline-list" @wheel.prevent="handleWheel">
      <div
        v-for="todo in parentTodos"
        :key="todo.id"
        @click="scrollToTodo(todo.id, $event)"
        class="outline-item"
        :class="{ 'completed': todo.completed, 'active': activeTodoId === todo.id }"
        :title="todo.content"
      >
        {{ todo.content }}
      </div>
    </div>
  </div>
</template>

<style>
.outline-container {
  background: var(--bg-app);
  padding: 12px 32px 10px;
  margin-top: -18px;
  margin-bottom: 8px;
  margin-left: -32px;
  margin-right: -32px;
  position: sticky;
  top: 0;
  z-index: 120;
}

.outline-list {
  display: flex;
  gap: 7px;
  padding: 8px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  overflow-x: auto;
  overflow-y: hidden;
}

.outline-item {
  flex-shrink: 0;
  padding: 7px 12px;
  cursor: pointer;
  transition: background-color 0.2s, color 0.2s, border-color 0.2s, transform 0.2s, box-shadow 0.2s;
  font-size: 14px;
  color: var(--text-primary);
  background: var(--bg-secondary);
  border-radius: 6px;
  white-space: nowrap;
  border: 1px solid var(--border-color);
  position: relative;
  z-index: 1;
}

.outline-item:hover {
  background: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.outline-item.active {
  background: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
  box-shadow: 0 0 0 3px rgba(79, 70, 229, 0.16);
  transform: translateY(-1px);
}

.outline-item.completed {
  background: #dcfce7;
  border-color: #86efac;
  color: #166534;
}

.outline-list {
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.outline-list::-webkit-scrollbar {
  display: none;
}

@media (prefers-color-scheme: dark) {
  .outline-item:hover {
    background: #6366f1;
    border-color: #6366f1;
  }

  .outline-item.completed {
    background: #14532d;
    border-color: #166534;
    color: #4ade80;
  }

  .outline-item.completed:hover {
    background: #166534;
    border-color: #22c55e;
  }

  .outline-item.active {
    background: #6366f1;
    border-color: #6366f1;
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.22);
  }
}

</style>
