import { useEffect, useRef } from 'react';
import { Animated, StyleSheet, Text } from 'react-native';
import { useTheme } from '../theme/useTheme';
import { tintedBanner } from '../theme/platformStyles';
import type { Colors } from '../theme/colors';

interface NewMenuToastProps {
  visible: boolean;
  onDismiss: () => void;
}

const DISPLAY_DURATION = 4000;
const ANIMATION_DURATION = 300;

export function NewMenuToast({ visible, onDismiss }: NewMenuToastProps) {
  const { colors } = useTheme();
  const translateY = useRef(new Animated.Value(-100)).current;
  const opacity = useRef(new Animated.Value(0)).current;

  useEffect(() => {
    if (visible) {
      Animated.parallel([
        Animated.timing(translateY, {
          toValue: 0,
          duration: ANIMATION_DURATION,
          useNativeDriver: true,
        }),
        Animated.timing(opacity, {
          toValue: 1,
          duration: ANIMATION_DURATION,
          useNativeDriver: true,
        }),
      ]).start();

      const timer = setTimeout(() => {
        Animated.parallel([
          Animated.timing(translateY, {
            toValue: -100,
            duration: ANIMATION_DURATION,
            useNativeDriver: true,
          }),
          Animated.timing(opacity, {
            toValue: 0,
            duration: ANIMATION_DURATION,
            useNativeDriver: true,
          }),
        ]).start(() => onDismiss());
      }, DISPLAY_DURATION);

      return () => clearTimeout(timer);
    }
  }, [visible, translateY, opacity, onDismiss]);

  if (!visible) return null;

  const styles = createStyles(colors);

  return (
    <Animated.View style={[styles.container, { transform: [{ translateY }], opacity }]}>
      <Text style={styles.text}>Neue Menüs verfügbar!</Text>
    </Animated.View>
  );
}

const createStyles = (c: Colors) =>
  StyleSheet.create({
    container: {
      position: 'absolute',
      top: 0,
      left: 16,
      right: 16,
      zIndex: 100,
      padding: 12,
      alignItems: 'center',
      ...tintedBanner(c, c.glassPrimary),
    },
    text: {
      fontSize: 14,
      fontWeight: '600',
      color: c.primary,
    },
  });
