import type {
  ButtonHTMLAttributes,
  InputHTMLAttributes,
  ReactNode,
  TextareaHTMLAttributes,
} from "react";

export function Button(
  props: ButtonHTMLAttributes<HTMLButtonElement> & {
    variant?: "primary" | "ghost" | "danger";
    size?: "md" | "sm" | "xs";
  },
): ReactNode;

export function Field(props: {
  label?: ReactNode;
  hint?: ReactNode;
  error?: ReactNode;
  className?: string;
  children?: ReactNode;
}): ReactNode;

export function Input(props: InputHTMLAttributes<HTMLInputElement>): ReactNode;

export function Textarea(props: TextareaHTMLAttributes<HTMLTextAreaElement>): ReactNode;

export function Modal(props: {
  title: string;
  subtitle?: string;
  eyebrow?: string;
  onClose: () => void;
  wide?: boolean;
  dismissOnOverlay?: boolean;
  children?: ReactNode;
  footer?: ReactNode;
}): ReactNode;

export function SegmentedControl(props: {
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
}): ReactNode;

export function Switch(props: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: ReactNode;
  description?: ReactNode;
  disabled?: boolean;
  className?: string;
}): ReactNode;
